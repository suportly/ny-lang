use inkwell::types::BasicType;
use inkwell::values::{BasicValueEnum, FunctionValue};
use inkwell::{AddressSpace, IntPredicate};

use crate::common::{CompileError, NyType, Span};
use crate::parser::ast::*;

use super::types::ny_to_llvm;
use super::{CodeGen, LoopFrame};

impl<'ctx> CodeGen<'ctx> {
    // ------------------------------------------------------------------
    // Compile statements
    // ------------------------------------------------------------------

    pub(super) fn compile_stmt(
        &mut self,
        stmt: &Stmt,
        function: &FunctionValue<'ctx>,
    ) -> Result<(), Vec<CompileError>> {
        match stmt {
            Stmt::VarDecl { name, ty, init, .. } => {
                let val = self.compile_expr(init, function)?.unwrap();
                let ny_ty = if let Some(t) = ty {
                    self.resolve_type_annotation(t)
                } else {
                    self.infer_expr_type(init)
                };

                // Coerce concrete type to dyn Trait fat pointer
                let val = if let NyType::DynTrait(ref trait_name) = ny_ty {
                    let concrete_ty = self.infer_expr_type(init);
                    let type_name = match &concrete_ty {
                        NyType::Pointer(inner) => match inner.as_ref() {
                            NyType::Struct { name, .. } => name.clone(),
                            _ => String::new(),
                        },
                        NyType::Struct { name, .. } => name.clone(),
                        _ => String::new(),
                    };
                    let vtable_key = format!("{}_for_{}", trait_name, type_name);
                    if let Some((vtable_ptr, _)) = self.vtables.get(&vtable_key) {
                        let ptr_ty = self.context.ptr_type(inkwell::AddressSpace::default());
                        let fat_ty = self.context.struct_type(&[ptr_ty.into(), ptr_ty.into()], false);
                        let fat_alloca = self.builder.build_alloca(fat_ty, "dyn_fat").unwrap();
                        // Store data pointer
                        let data_gep = self.builder.build_struct_gep(fat_ty, fat_alloca, 0, "dyn_data").unwrap();
                        self.builder.build_store(data_gep, val.into_pointer_value()).unwrap();
                        // Store vtable pointer
                        let vtable_gep = self.builder.build_struct_gep(fat_ty, fat_alloca, 1, "dyn_vtable").unwrap();
                        self.builder.build_store(vtable_gep, *vtable_ptr).unwrap();
                        // Load the fat pointer struct
                        self.builder.build_load(fat_ty, fat_alloca, "dyn_val").unwrap()
                    } else {
                        val
                    }
                } else {
                    val
                };

                let llvm_ty = ny_to_llvm(self.context, &ny_ty);
                let alloca = self.builder.build_alloca(llvm_ty, name).unwrap();
                self.builder.build_store(alloca, val).unwrap();
                self.variables.insert(name.clone(), (alloca, ny_ty.clone()));

                // Fix Vec<T> elem_size: vec_new() hardcodes elem_size=8,
                // correct it based on the actual element type T.
                if let NyType::Vec(ref elem_ty) = ny_ty {
                    let elem_llvm = ny_to_llvm(self.context, elem_ty);
                    let actual_elem_size = elem_llvm.size_of().unwrap();
                    let vec_struct_ty = ny_to_llvm(self.context, &ny_ty).into_struct_type();

                    // Update elem_size field (index 3) to actual value
                    let es_ptr = self
                        .builder
                        .build_struct_gep(vec_struct_ty, alloca, 3, "vec_fix_es")
                        .unwrap();
                    self.builder.build_store(es_ptr, actual_elem_size).unwrap();

                    // If actual elem_size > 8, the initial buffer (cap * 8) is too small.
                    // Realloc to cap * actual_elem_size.
                    let eight = self.context.i64_type().const_int(8, false);
                    let needs_realloc = self
                        .builder
                        .build_int_compare(
                            IntPredicate::UGT,
                            actual_elem_size,
                            eight,
                            "vec_needs_realloc",
                        )
                        .unwrap();

                    let realloc_bb = self
                        .context
                        .append_basic_block(*function, "vec_init_realloc");
                    let done_bb = self.context.append_basic_block(*function, "vec_init_done");
                    self.builder
                        .build_conditional_branch(needs_realloc, realloc_bb, done_bb)
                        .unwrap();

                    self.builder.position_at_end(realloc_bb);
                    let cap_ptr = self
                        .builder
                        .build_struct_gep(vec_struct_ty, alloca, 2, "vec_cap_gep")
                        .unwrap();
                    let cap = self
                        .builder
                        .build_load(self.context.i64_type(), cap_ptr, "vec_cap")
                        .unwrap()
                        .into_int_value();
                    let correct_size = self
                        .builder
                        .build_int_mul(cap, actual_elem_size, "vec_correct_size")
                        .unwrap();
                    let data_ptr_gep = self
                        .builder
                        .build_struct_gep(vec_struct_ty, alloca, 0, "vec_data_gep")
                        .unwrap();
                    let old_data = self
                        .builder
                        .build_load(
                            self.context.ptr_type(AddressSpace::default()),
                            data_ptr_gep,
                            "vec_old_data",
                        )
                        .unwrap()
                        .into_pointer_value();
                    let realloc_fn = self.get_or_declare_realloc();
                    let new_data = self
                        .builder
                        .build_call(
                            realloc_fn,
                            &[old_data.into(), correct_size.into()],
                            "vec_realloc_data",
                        )
                        .unwrap()
                        .try_as_basic_value()
                        .basic()
                        .unwrap()
                        .into_pointer_value();
                    self.builder.build_store(data_ptr_gep, new_data).unwrap();
                    self.builder.build_unconditional_branch(done_bb).unwrap();

                    self.builder.position_at_end(done_bb);
                }

                // If this is a capturing lambda, transfer closure info to var name
                if let Expr::Lambda { .. } = init {
                    // Find which lambda was just compiled (most recent one)
                    let recent_lambda: Option<String> = self
                        .closure_captures
                        .keys()
                        .filter(|k| k.starts_with("__lambda_"))
                        .max()
                        .cloned();
                    if let Some(lambda_key) = recent_lambda {
                        if let Some(info) = self.closure_captures.remove(&lambda_key) {
                            self.closure_captures.insert(name.clone(), info);
                        }
                    }
                }

                Ok(())
            }
            Stmt::ConstDecl {
                name, ty, value, ..
            } => {
                let val = self.compile_expr(value, function)?.unwrap();
                let ny_ty = if let Some(t) = ty {
                    self.resolve_type_annotation(t)
                } else {
                    self.infer_expr_type(value)
                };
                let llvm_ty = ny_to_llvm(self.context, &ny_ty);
                let alloca = self.builder.build_alloca(llvm_ty, name).unwrap();
                self.builder.build_store(alloca, val).unwrap();
                self.variables.insert(name.clone(), (alloca, ny_ty));
                Ok(())
            }
            Stmt::Assign { target, value, .. } => {
                let val = self.compile_expr(value, function)?.unwrap();
                self.compile_assign_target(target, val, function)?;
                Ok(())
            }
            Stmt::ExprStmt { expr, .. } => {
                self.compile_expr(expr, function)?;
                Ok(())
            }
            Stmt::Return { value, .. } => {
                // Evaluate the return value first
                let ret_val = if let Some(val_expr) = value {
                    self.compile_expr(val_expr, function)?
                } else {
                    None
                };

                // Emit all deferred expressions in LIFO order before returning
                let defers: Vec<(Expr, FunctionValue<'ctx>)> =
                    self.defer_stack.iter().rev().cloned().collect();
                for (defer_body, defer_fn) in &defers {
                    self.compile_expr(defer_body, defer_fn)?;
                }

                // Pop trace stack before return (debug only)
                if self.opt_level < 2 {
                    let trace_pop = self.get_or_declare_ny_trace_pop();
                    self.builder.build_call(trace_pop, &[], "").unwrap();
                }

                if let Some(v) = ret_val {
                    // Coerce to dyn Trait if function returns dyn Trait
                    let fn_name = function.get_name().to_str().unwrap_or("").to_string();
                    let ret_ty = self.functions.get(&fn_name).map(|(_, _, rt)| rt.clone());
                    let v = if let Some(NyType::DynTrait(ref trait_name)) = ret_ty {
                        let val_expr = value.as_ref().unwrap();
                        let concrete_ty = self.infer_expr_type(val_expr);
                        let type_name = match &concrete_ty {
                            NyType::Pointer(inner) => match inner.as_ref() {
                                NyType::Struct { name, .. } => name.clone(),
                                _ => String::new(),
                            },
                            _ => String::new(),
                        };
                        let vtable_key = format!("{}_for_{}", trait_name, type_name);
                        if let Some((vtable_ptr, _)) = self.vtables.get(&vtable_key) {
                            let ptr_ty = self.context.ptr_type(inkwell::AddressSpace::default());
                            let fat_ty = self.context.struct_type(&[ptr_ty.into(), ptr_ty.into()], false);
                            let fat_alloca = self.builder.build_alloca(fat_ty, "ret_dyn").unwrap();
                            let data_gep = self.builder.build_struct_gep(fat_ty, fat_alloca, 0, "ret_data").unwrap();
                            self.builder.build_store(data_gep, v.into_pointer_value()).unwrap();
                            let vtable_gep = self.builder.build_struct_gep(fat_ty, fat_alloca, 1, "ret_vtbl").unwrap();
                            self.builder.build_store(vtable_gep, *vtable_ptr).unwrap();
                            self.builder.build_load(fat_ty, fat_alloca, "ret_fat").unwrap()
                        } else {
                            v
                        }
                    } else {
                        v
                    };
                    self.builder.build_return(Some(&v)).unwrap();
                } else {
                    self.builder.build_return(None).unwrap();
                }
                Ok(())
            }

            // ---- While loop (now with loop_stack for break/continue) ----
            Stmt::While {
                condition, body, ..
            } => {
                let cond_bb = self.context.append_basic_block(*function, "while_cond");
                let body_bb = self.context.append_basic_block(*function, "while_body");
                let exit_bb = self.context.append_basic_block(*function, "while_exit");

                self.builder.build_unconditional_branch(cond_bb).unwrap();

                // Push loop frame (break -> exit_bb, continue -> cond_bb)
                self.loop_stack.push(LoopFrame {
                    break_bb: exit_bb,
                    continue_bb: cond_bb,
                });

                // Condition
                self.builder.position_at_end(cond_bb);
                let cond_val = self.compile_expr(condition, function)?.unwrap();
                self.builder
                    .build_conditional_branch(cond_val.into_int_value(), body_bb, exit_bb)
                    .unwrap();

                // Body
                self.builder.position_at_end(body_bb);
                self.compile_expr(body, function)?;
                if self
                    .builder
                    .get_insert_block()
                    .unwrap()
                    .get_terminator()
                    .is_none()
                {
                    self.builder.build_unconditional_branch(cond_bb).unwrap();
                }

                self.loop_stack.pop();

                // Exit
                self.builder.position_at_end(exit_bb);
                Ok(())
            }

            // ---- For-range loop ----
            Stmt::ForRange {
                var,
                start,
                end,
                inclusive,
                body,
                ..
            } => {
                // Evaluate start and end
                let start_val = self
                    .compile_expr(start, function)?
                    .unwrap()
                    .into_int_value();
                let end_val = self.compile_expr(end, function)?.unwrap().into_int_value();

                // Allocate the loop variable
                let i32_ty = self.context.i32_type();
                let loop_var = self.builder.build_alloca(i32_ty, var).unwrap();
                self.builder.build_store(loop_var, start_val).unwrap();
                self.variables.insert(var.clone(), (loop_var, NyType::I32));

                let cond_bb = self.context.append_basic_block(*function, "for_cond");
                let body_bb = self.context.append_basic_block(*function, "for_body");
                let inc_bb = self.context.append_basic_block(*function, "for_inc");
                let exit_bb = self.context.append_basic_block(*function, "for_exit");

                self.builder.build_unconditional_branch(cond_bb).unwrap();

                // Push loop frame (break -> exit_bb, continue -> inc_bb)
                self.loop_stack.push(LoopFrame {
                    break_bb: exit_bb,
                    continue_bb: inc_bb,
                });

                // Condition block
                self.builder.position_at_end(cond_bb);
                let current_val = self
                    .builder
                    .build_load(i32_ty, loop_var, "loop_var")
                    .unwrap()
                    .into_int_value();
                let cmp = if *inclusive {
                    // <=
                    self.builder
                        .build_int_compare(IntPredicate::SLE, current_val, end_val, "for_cond")
                        .unwrap()
                } else {
                    // <
                    self.builder
                        .build_int_compare(IntPredicate::SLT, current_val, end_val, "for_cond")
                        .unwrap()
                };
                self.builder
                    .build_conditional_branch(cmp, body_bb, exit_bb)
                    .unwrap();

                // Body block
                self.builder.position_at_end(body_bb);
                self.compile_expr(body, function)?;
                if self
                    .builder
                    .get_insert_block()
                    .unwrap()
                    .get_terminator()
                    .is_none()
                {
                    self.builder.build_unconditional_branch(inc_bb).unwrap();
                }

                // Increment block
                self.builder.position_at_end(inc_bb);
                let inc_val = self
                    .builder
                    .build_load(i32_ty, loop_var, "loop_var")
                    .unwrap()
                    .into_int_value();
                let next_val = self
                    .builder
                    .build_int_add(inc_val, i32_ty.const_int(1, false), "inc")
                    .unwrap();
                self.builder.build_store(loop_var, next_val).unwrap();
                self.builder.build_unconditional_branch(cond_bb).unwrap();

                self.loop_stack.pop();

                // Exit block
                self.builder.position_at_end(exit_bb);
                Ok(())
            }

            // ---- ForIn (desugar: for var in collection → index loop) ----
            Stmt::ForIn {
                var,
                collection,
                body,
                ..
            } => {
                let coll_ty = self.infer_expr_type(collection);
                let (elem_ty, coll_len) = match &coll_ty {
                    NyType::Array { elem, size } => (
                        *elem.clone(),
                        self.context.i32_type().const_int(*size as u64, false),
                    ),
                    NyType::Slice(_) | NyType::Vec(_) => {
                        // Get len dynamically
                        let coll_val = self.compile_expr(collection, function)?.unwrap();
                        let sv = coll_val.into_struct_value();
                        let len = self
                            .builder
                            .build_extract_value(sv, 1, "forin_len")
                            .unwrap()
                            .into_int_value();
                        let len_i32 = self
                            .builder
                            .build_int_truncate(len, self.context.i32_type(), "len_i32")
                            .unwrap();
                        let elem = match &coll_ty {
                            NyType::Slice(e) => *e.clone(),
                            NyType::Vec(e) => *e.clone(),
                            _ => NyType::I32,
                        };
                        (elem, len_i32)
                    }
                    _ => {
                        return Err(vec![CompileError::type_error(
                            format!("cannot iterate over '{}'", coll_ty),
                            collection.span(),
                        )]);
                    }
                };

                let elem_llvm = ny_to_llvm(self.context, &elem_ty);
                let i32_ty = self.context.i32_type();

                // Allocate loop index
                let idx_alloca = self.builder.build_alloca(i32_ty, "__forin_idx").unwrap();
                self.builder
                    .build_store(idx_alloca, i32_ty.const_zero())
                    .unwrap();

                // Allocate element variable
                let var_alloca = self.builder.build_alloca(elem_llvm, var).unwrap();
                self.variables
                    .insert(var.clone(), (var_alloca, elem_ty.clone()));

                let cond_bb = self.context.append_basic_block(*function, "forin_cond");
                let body_bb = self.context.append_basic_block(*function, "forin_body");
                let inc_bb = self.context.append_basic_block(*function, "forin_inc");
                let exit_bb = self.context.append_basic_block(*function, "forin_exit");

                self.builder.build_unconditional_branch(cond_bb).unwrap();

                self.loop_stack.push(LoopFrame {
                    break_bb: exit_bb,
                    continue_bb: inc_bb,
                });

                // Condition: idx < len
                self.builder.position_at_end(cond_bb);
                let idx = self
                    .builder
                    .build_load(i32_ty, idx_alloca, "idx")
                    .unwrap()
                    .into_int_value();
                let cmp = self
                    .builder
                    .build_int_compare(IntPredicate::SLT, idx, coll_len, "forin_cmp")
                    .unwrap();
                self.builder
                    .build_conditional_branch(cmp, body_bb, exit_bb)
                    .unwrap();

                // Body: load element, execute body
                self.builder.position_at_end(body_bb);
                // Get element from collection[idx]
                match &coll_ty {
                    NyType::Array { elem, size } => {
                        let arr_ptr = self.compile_expr_as_ptr(collection, function)?;
                        let elem_llvm_inner = ny_to_llvm(self.context, elem);
                        let arr_llvm_ty = elem_llvm_inner.array_type(*size as u32);
                        let cur_idx = self
                            .builder
                            .build_load(i32_ty, idx_alloca, "cur_idx")
                            .unwrap()
                            .into_int_value();
                        let idx_i64 = self
                            .builder
                            .build_int_z_extend_or_bit_cast(
                                cur_idx,
                                self.context.i64_type(),
                                "idx64",
                            )
                            .unwrap();
                        let zero = self.context.i64_type().const_zero();
                        let gep = unsafe {
                            self.builder
                                .build_in_bounds_gep(
                                    arr_llvm_ty,
                                    arr_ptr,
                                    &[zero, idx_i64],
                                    "forin_gep",
                                )
                                .unwrap()
                        };
                        let elem_val = self.builder.build_load(elem_llvm, gep, "elem").unwrap();
                        self.builder.build_store(var_alloca, elem_val).unwrap();
                    }
                    NyType::Slice(_) | NyType::Vec(_) => {
                        // For slices/vecs, get the data pointer
                        let coll_val = self.compile_expr(collection, function)?.unwrap();
                        let sv = coll_val.into_struct_value();
                        let data_ptr = self
                            .builder
                            .build_extract_value(sv, 0, "data_ptr")
                            .unwrap()
                            .into_pointer_value();
                        let cur_idx = self
                            .builder
                            .build_load(i32_ty, idx_alloca, "cur_idx")
                            .unwrap()
                            .into_int_value();
                        let idx_i64 = self
                            .builder
                            .build_int_z_extend_or_bit_cast(
                                cur_idx,
                                self.context.i64_type(),
                                "idx64",
                            )
                            .unwrap();
                        let gep = unsafe {
                            self.builder
                                .build_in_bounds_gep(elem_llvm, data_ptr, &[idx_i64], "forin_gep")
                                .unwrap()
                        };
                        let elem_val = self.builder.build_load(elem_llvm, gep, "elem").unwrap();
                        self.builder.build_store(var_alloca, elem_val).unwrap();
                    }
                    _ => {}
                }

                self.compile_expr(body, function)?;
                if self
                    .builder
                    .get_insert_block()
                    .unwrap()
                    .get_terminator()
                    .is_none()
                {
                    self.builder.build_unconditional_branch(inc_bb).unwrap();
                }

                // Increment
                self.builder.position_at_end(inc_bb);
                let cur = self
                    .builder
                    .build_load(i32_ty, idx_alloca, "cur")
                    .unwrap()
                    .into_int_value();
                let next = self
                    .builder
                    .build_int_add(cur, i32_ty.const_int(1, false), "next")
                    .unwrap();
                self.builder.build_store(idx_alloca, next).unwrap();
                self.builder.build_unconditional_branch(cond_bb).unwrap();

                self.loop_stack.pop();
                self.builder.position_at_end(exit_bb);
                Ok(())
            }

            // ---- Break ----
            Stmt::Break { .. } => {
                if let Some(frame) = self.loop_stack.last() {
                    let break_bb = frame.break_bb;
                    self.builder.build_unconditional_branch(break_bb).unwrap();
                }
                Ok(())
            }

            // ---- Continue ----
            Stmt::Continue { .. } => {
                if let Some(frame) = self.loop_stack.last() {
                    let continue_bb = frame.continue_bb;
                    self.builder
                        .build_unconditional_branch(continue_bb)
                        .unwrap();
                }
                Ok(())
            }

            Stmt::WhileLet {
                pattern,
                expr: match_expr,
                body: while_body,
                span: while_span,
            } => {
                // Desugar: loop { match expr { Pattern => body, _ => break } }
                let loop_body_bb = self.context.append_basic_block(*function, "whilelet_body");
                let exit_bb = self.context.append_basic_block(*function, "whilelet_exit");

                self.builder
                    .build_unconditional_branch(loop_body_bb)
                    .unwrap();
                self.loop_stack.push(LoopFrame {
                    break_bb: exit_bb,
                    continue_bb: loop_body_bb,
                });

                self.builder.position_at_end(loop_body_bb);

                // Build: match expr { pattern => body, _ => break }
                let break_body = Expr::Block {
                    stmts: vec![Stmt::Break { span: *while_span }],
                    tail_expr: None,
                    span: *while_span,
                };
                let match_ast = Expr::Match {
                    subject: Box::new(match_expr.clone()),
                    arms: vec![
                        MatchArm {
                            pattern: pattern.clone(),
                            body: while_body.clone(),
                        },
                        MatchArm {
                            pattern: Pattern::Wildcard(Span::empty(0)),
                            body: break_body,
                        },
                    ],
                    span: *while_span,
                };
                self.compile_expr(&match_ast, function)?;

                if self
                    .builder
                    .get_insert_block()
                    .unwrap()
                    .get_terminator()
                    .is_none()
                {
                    self.builder
                        .build_unconditional_branch(loop_body_bb)
                        .unwrap();
                }

                self.loop_stack.pop();
                self.builder.position_at_end(exit_bb);
                Ok(())
            }

            Stmt::IfLet {
                pattern,
                expr: match_expr,
                then_body,
                else_body,
                ..
            } => {
                // Desugar if let to: match expr { pattern => then, _ => else }
                let wildcard_body = if let Some(eb) = else_body {
                    eb.clone()
                } else {
                    Expr::Block {
                        stmts: Vec::new(),
                        tail_expr: None,
                        span: Span::empty(0),
                    }
                };
                let match_expr_ast = Expr::Match {
                    subject: Box::new(match_expr.clone()),
                    arms: vec![
                        MatchArm {
                            pattern: pattern.clone(),
                            body: then_body.clone(),
                        },
                        MatchArm {
                            pattern: Pattern::Wildcard(Span::empty(0)),
                            body: wildcard_body,
                        },
                    ],
                    span: Span::empty(0),
                };
                self.compile_expr(&match_expr_ast, function)?;
                Ok(())
            }

            Stmt::Loop { body, .. } => {
                let body_bb = self.context.append_basic_block(*function, "loop_body");
                let exit_bb = self.context.append_basic_block(*function, "loop_exit");

                self.builder.build_unconditional_branch(body_bb).unwrap();

                self.loop_stack.push(LoopFrame {
                    break_bb: exit_bb,
                    continue_bb: body_bb,
                });

                self.builder.position_at_end(body_bb);
                self.compile_expr(body, function)?;
                if self
                    .builder
                    .get_insert_block()
                    .unwrap()
                    .get_terminator()
                    .is_none()
                {
                    self.builder.build_unconditional_branch(body_bb).unwrap();
                }

                self.loop_stack.pop();
                self.builder.position_at_end(exit_bb);
                Ok(())
            }

            Stmt::ForMap { key_var, val_var, map_expr, body, .. } => {
                let map_ptr = self.compile_expr(map_expr, function)?.unwrap();
                let i32_ty = self.context.i32_type();
                let i64_ty = self.context.i64_type();

                // Get map length
                let map_len_fn = self.get_or_declare_c_fn(
                    "ny_map_len",
                    i64_ty.fn_type(&[self.context.ptr_type(inkwell::AddressSpace::default()).into()], false),
                );
                let len = self.builder.build_call(map_len_fn, &[map_ptr.into()], "map_len")
                    .unwrap().try_as_basic_value().basic().unwrap().into_int_value();
                let len_i32 = self.builder.build_int_truncate(len, i32_ty, "len32").unwrap();

                // Index variable
                let idx_alloca = self.builder.build_alloca(i32_ty, "map_idx").unwrap();
                self.builder.build_store(idx_alloca, i32_ty.const_int(0, false)).unwrap();

                let cond_bb = self.context.append_basic_block(*function, "formap_cond");
                let body_bb = self.context.append_basic_block(*function, "formap_body");
                let exit_bb = self.context.append_basic_block(*function, "formap_exit");

                self.builder.build_unconditional_branch(cond_bb).unwrap();
                self.builder.position_at_end(cond_bb);

                let idx = self.builder.build_load(i32_ty, idx_alloca, "idx").unwrap().into_int_value();
                let cmp = self.builder.build_int_compare(IntPredicate::SLT, idx, len_i32, "cmp").unwrap();
                self.builder.build_conditional_branch(cmp, body_bb, exit_bb).unwrap();

                self.builder.position_at_end(body_bb);

                let ptr_ty = self.context.ptr_type(inkwell::AddressSpace::default());
                let idx_i64 = self.builder.build_int_z_extend(idx, i64_ty, "idx64").unwrap();

                // ny_map_key_at(map, index, &out_len) -> *u8
                let out_len_alloca = self.builder.build_alloca(i64_ty, "key_len").unwrap();
                let map_key_at_fn = self.get_or_declare_c_fn(
                    "ny_map_key_at",
                    ptr_ty.fn_type(&[ptr_ty.into(), i64_ty.into(), ptr_ty.into()], false),
                );
                let key_ptr = self.builder.build_call(
                    map_key_at_fn,
                    &[map_ptr.into(), idx_i64.into(), out_len_alloca.into()],
                    "key_ptr",
                ).unwrap().try_as_basic_value().basic().unwrap();
                let key_len = self.builder.build_load(i64_ty, out_len_alloca, "key_len_v").unwrap();

                // Build str struct {ptr, len}
                let str_struct_ty = ny_to_llvm(self.context, &NyType::Str).into_struct_type();
                let key_s0 = self.builder.build_insert_value(str_struct_ty.get_undef(), key_ptr, 0, "ks_p").unwrap();
                let key_val = self.builder.build_insert_value(key_s0.into_struct_value(), key_len, 1, "ks_l").unwrap();

                // ny_map_get(map, key_ptr, key_len) -> i64 (value)
                let map_get_fn = self.get_or_declare_c_fn(
                    "ny_map_get",
                    i64_ty.fn_type(&[ptr_ty.into(), ptr_ty.into(), i64_ty.into()], false),
                );
                let val_i64 = self.builder.build_call(
                    map_get_fn,
                    &[map_ptr.into(), key_ptr.into(), key_len.into()],
                    "val_i64",
                ).unwrap().try_as_basic_value().basic().unwrap().into_int_value();
                let val_val = self.builder.build_int_truncate(val_i64, i32_ty, "val_i32").unwrap();

                // Declare key and value variables
                let outer_vars = self.variables.clone();
                let str_llvm_ty = ny_to_llvm(self.context, &NyType::Str);
                let key_alloca = self.builder.build_alloca(str_llvm_ty, key_var).unwrap();
                self.builder.build_store(key_alloca, key_val.into_struct_value()).unwrap();
                self.variables.insert(key_var.clone(), (key_alloca, NyType::Str));

                let val_alloca = self.builder.build_alloca(i32_ty, val_var).unwrap();
                self.builder.build_store(val_alloca, val_val).unwrap();
                self.variables.insert(val_var.clone(), (val_alloca, NyType::I32));

                self.loop_stack.push(LoopFrame { break_bb: exit_bb, continue_bb: cond_bb });
                self.compile_expr(body, function)?;
                self.loop_stack.pop();
                self.variables = outer_vars;

                // Increment index
                if self.builder.get_insert_block().unwrap().get_terminator().is_none() {
                    let next_idx = self.builder.build_int_add(idx, i32_ty.const_int(1, false), "next").unwrap();
                    self.builder.build_store(idx_alloca, next_idx).unwrap();
                    self.builder.build_unconditional_branch(cond_bb).unwrap();
                }

                self.builder.position_at_end(exit_bb);
                Ok(())
            }

            Stmt::Select { arms, .. } => {
                let i32_ty = self.context.i32_type();
                let i64_ty = self.context.i64_type();

                // Pre-compile channel pointers and element types
                let mut ch_ptrs = Vec::new();
                let mut elem_types = Vec::new();
                for arm in arms.iter() {
                    if let Expr::MethodCall { object, .. } = &arm.channel {
                        let obj_ty = self.infer_expr_type(object);
                        let elem = match &obj_ty {
                            NyType::Chan(e) => *e.clone(),
                            _ => NyType::I32,
                        };
                        let ptr = self.compile_expr(object, function)?.unwrap();
                        ch_ptrs.push(ptr);
                        elem_types.push(elem);
                    } else {
                        ch_ptrs.push(self.compile_expr(&arm.channel, function)?.unwrap());
                        elem_types.push(NyType::I32);
                    }
                }

                // Allocate receive buffer (8 bytes fits i32/i64/f64/ptr)
                let recv_buf = self.builder.build_alloca(i64_ty, "sel_buf").unwrap();
                let try_recv_fn = self.get_or_declare_ny_chan_try_recv();
                let sleep_fn = self.get_or_declare_c_fn(
                    "usleep",
                    i32_ty.fn_type(&[i32_ty.into()], false),
                );

                let poll_bb = self.context.append_basic_block(*function, "sel_poll");
                let merge_bb = self.context.append_basic_block(*function, "sel_merge");
                self.builder.build_unconditional_branch(poll_bb).unwrap();

                // Build chain: poll → try0 → try1 → ... → sleep → poll
                self.builder.position_at_end(poll_bb);

                let mut prev_fail_bb = poll_bb;
                for (i, arm) in arms.iter().enumerate() {
                    // If i > 0, we need to be in the "not ready" block from previous try
                    if i > 0 {
                        self.builder.position_at_end(prev_fail_bb);
                    }

                    let got = self.builder.build_call(
                        try_recv_fn,
                        &[ch_ptrs[i].into(), recv_buf.into()],
                        &format!("got_{}", i),
                    ).unwrap().try_as_basic_value().basic().unwrap().into_int_value();

                    let is_ready = self.builder.build_int_compare(
                        IntPredicate::NE, got, i32_ty.const_int(0, false), &format!("rdy_{}", i),
                    ).unwrap();

                    let arm_bb = self.context.append_basic_block(*function, &format!("sel_arm_{}", i));
                    let fail_bb = self.context.append_basic_block(*function, &format!("sel_fail_{}", i));
                    self.builder.build_conditional_branch(is_ready, arm_bb, fail_bb).unwrap();

                    // Compile arm body
                    self.builder.position_at_end(arm_bb);
                    let elem_llvm = ny_to_llvm(self.context, &elem_types[i]);
                    let recv_val = self.builder.build_load(elem_llvm, recv_buf, &format!("val_{}", i)).unwrap();
                    let var_alloca = self.builder.build_alloca(elem_llvm, &arm.var).unwrap();
                    self.builder.build_store(var_alloca, recv_val).unwrap();
                    let outer_vars = self.variables.clone();
                    self.variables.insert(arm.var.clone(), (var_alloca, elem_types[i].clone()));
                    self.compile_expr(&arm.body, function)?;
                    self.variables = outer_vars;
                    if self.builder.get_insert_block().unwrap().get_terminator().is_none() {
                        self.builder.build_unconditional_branch(merge_bb).unwrap();
                    }

                    prev_fail_bb = fail_bb;
                }

                // All channels empty: sleep 1ms, then retry
                self.builder.position_at_end(prev_fail_bb);
                self.builder.build_call(sleep_fn, &[i32_ty.const_int(1000, false).into()], "").unwrap();
                self.builder.build_unconditional_branch(poll_bb).unwrap();

                self.builder.position_at_end(merge_bb);
                Ok(())
            }

            Stmt::Defer { body, .. } => {
                // Push the deferred expression onto the stack; it will be emitted
                // when the function returns or the scope exits.
                self.defer_stack.push((body.clone(), *function));
                Ok(())
            }

            Stmt::TupleDestructure { names, init, .. } => {
                let val = self.compile_expr(init, function)?.unwrap();
                let struct_val = val.into_struct_value();
                let tuple_ty = self.infer_expr_type(init);
                let elem_types = match &tuple_ty {
                    NyType::Tuple(elems) => elems.clone(),
                    _ => vec![NyType::I32; names.len()],
                };

                for (i, name) in names.iter().enumerate() {
                    let elem_val = self
                        .builder
                        .build_extract_value(struct_val, i as u32, name)
                        .unwrap();
                    let elem_ty = elem_types.get(i).cloned().unwrap_or(NyType::I32);
                    let llvm_ty = ny_to_llvm(self.context, &elem_ty);
                    let alloca = self.builder.build_alloca(llvm_ty, name).unwrap();
                    self.builder.build_store(alloca, elem_val).unwrap();
                    self.variables.insert(name.clone(), (alloca, elem_ty));
                }
                Ok(())
            }
        }
    }

    // ------------------------------------------------------------------
    // AssignTarget compilation
    // ------------------------------------------------------------------

    pub(super) fn compile_assign_target(
        &mut self,
        target: &AssignTarget,
        val: BasicValueEnum<'ctx>,
        function: &FunctionValue<'ctx>,
    ) -> Result<(), Vec<CompileError>> {
        match target {
            AssignTarget::Var(name) => {
                if let Some((ptr, _)) = self.variables.get(name) {
                    self.builder.build_store(*ptr, val).unwrap();
                }
                Ok(())
            }
            AssignTarget::Index(object_expr, index_expr) => {
                let obj_ty = self.infer_expr_type(object_expr);
                let idx_val = self
                    .compile_expr(index_expr, function)?
                    .unwrap()
                    .into_int_value();

                match &obj_ty {
                    NyType::Array { elem, size } => {
                        let elem_llvm = ny_to_llvm(self.context, elem);
                        let arr_llvm = elem_llvm.array_type(*size as u32);
                        let arr_ptr = self.compile_expr_as_ptr(object_expr, function)?;

                        let arr_len = self.context.i64_type().const_int(*size as u64, false);
                        let idx_i64 = self
                            .builder
                            .build_int_z_extend_or_bit_cast(
                                idx_val,
                                self.context.i64_type(),
                                "idx_ext",
                            )
                            .unwrap();
                        self.build_bounds_check(idx_i64, arr_len, function);

                        let zero = self.context.i64_type().const_zero();
                        let gep = unsafe {
                            self.builder
                                .build_in_bounds_gep(
                                    arr_llvm,
                                    arr_ptr,
                                    &[zero, idx_i64],
                                    "idx_store_ptr",
                                )
                                .unwrap()
                        };
                        self.builder.build_store(gep, val).unwrap();
                    }
                    _ => {
                        return Err(vec![CompileError::type_error(
                            "cannot index non-array type for assignment".to_string(),
                            object_expr.span(),
                        )]);
                    }
                }
                Ok(())
            }
            AssignTarget::Field(object_expr, field_name) => {
                let obj_ty = self.infer_expr_type(object_expr);

                let (struct_name, struct_fields, is_pointer) = match &obj_ty {
                    NyType::Struct { name, fields } => (name.clone(), fields.clone(), false),
                    NyType::Pointer(inner) => match inner.as_ref() {
                        NyType::Struct { name, fields } => (name.clone(), fields.clone(), true),
                        _ => {
                            return Err(vec![CompileError::type_error(
                                "field assign on non-struct pointer".to_string(),
                                object_expr.span(),
                            )]);
                        }
                    },
                    _ => {
                        return Err(vec![CompileError::type_error(
                            format!("field assign on non-struct type '{}'", obj_ty),
                            object_expr.span(),
                        )]);
                    }
                };

                let field_idx = struct_fields
                    .iter()
                    .position(|(n, _)| n == field_name)
                    .unwrap_or(0) as u32;
                let struct_llvm_ty =
                    self.get_or_create_llvm_struct_type(&struct_name, &struct_fields);

                let struct_ptr = if is_pointer {
                    let ptr_val = self.compile_expr(object_expr, function)?.unwrap();
                    ptr_val.into_pointer_value()
                } else {
                    self.compile_expr_as_ptr(object_expr, function)?
                };

                let field_ptr = self
                    .builder
                    .build_struct_gep(
                        struct_llvm_ty,
                        struct_ptr,
                        field_idx,
                        &format!("{}_assign_ptr", field_name),
                    )
                    .unwrap();
                self.builder.build_store(field_ptr, val).unwrap();
                Ok(())
            }
            AssignTarget::Deref(operand_expr) => {
                let ptr_val = self.compile_expr(operand_expr, function)?.unwrap();
                let ptr = ptr_val.into_pointer_value();
                self.builder.build_store(ptr, val).unwrap();
                Ok(())
            }
        }
    }

    // ------------------------------------------------------------------
    // Bounds checking for array index operations
    // ------------------------------------------------------------------

    pub(super) fn build_bounds_check(
        &self,
        index: inkwell::values::IntValue<'ctx>,
        length: inkwell::values::IntValue<'ctx>,
        function: &FunctionValue<'ctx>,
    ) {
        // Skip bounds checking at -O2+ for performance (release mode)
        if self.opt_level >= 2 {
            return;
        }

        let in_bounds = self
            .builder
            .build_int_compare(IntPredicate::ULT, index, length, "bounds_check")
            .unwrap();

        let ok_bb = self.context.append_basic_block(*function, "bounds_ok");
        let fail_bb = self.context.append_basic_block(*function, "bounds_fail");

        self.builder
            .build_conditional_branch(in_bounds, ok_bb, fail_bb)
            .unwrap();

        // Fail block: print error message with index and length, then exit(1)
        self.builder.position_at_end(fail_bb);
        let fprintf_fn = self.get_or_declare_fprintf();
        let stderr_global = self.get_or_declare_stderr();
        let stderr_val = self
            .builder
            .build_load(
                self.context.ptr_type(AddressSpace::default()),
                stderr_global.as_pointer_value(),
                "stderr",
            )
            .unwrap();
        let fmt = self
            .builder
            .build_global_string_ptr(
                "panic: index out of bounds: index %ld, length %ld\n",
                "bounds_fmt",
            )
            .unwrap();
        // Extend index and length to i64 for printing
        let idx_i64 = self
            .builder
            .build_int_z_extend_or_bit_cast(index, self.context.i64_type(), "idx_print")
            .unwrap();
        let len_i64 = self
            .builder
            .build_int_z_extend_or_bit_cast(length, self.context.i64_type(), "len_print")
            .unwrap();
        self.builder
            .build_call(
                fprintf_fn,
                &[
                    stderr_val.into(),
                    fmt.as_pointer_value().into(),
                    idx_i64.into(),
                    len_i64.into(),
                ],
                "",
            )
            .unwrap();
        // Print stack trace before exiting
        let trace_print = self.get_or_declare_ny_trace_print();
        self.builder.build_call(trace_print, &[], "").unwrap();

        let exit_fn = self.get_or_declare_exit();
        self.builder
            .build_call(
                exit_fn,
                &[self.context.i32_type().const_int(1, false).into()],
                "",
            )
            .unwrap();
        self.builder.build_unreachable().unwrap();

        // Continue from ok block
        self.builder.position_at_end(ok_bb);
    }
}
