use inkwell::basic_block::BasicBlock;
use inkwell::types::{BasicType, BasicTypeEnum};
use inkwell::values::{BasicValueEnum, FunctionValue, PointerValue};
use inkwell::{AddressSpace, IntPredicate};

use crate::common::{CompileError, NyType};
use crate::parser::ast::*;

use super::types::{ny_to_llvm, str_type};
use super::CodeGen;

/// Find free variables in an expression (identifiers not in the given bound set)
pub(super) fn find_free_vars(expr: &Expr, bound: &[String]) -> Vec<String> {
    let mut free = Vec::new();
    find_free_vars_inner(expr, bound, &mut free);
    free.sort();
    free.dedup();
    free
}

fn find_free_vars_inner(expr: &Expr, bound: &[String], free: &mut Vec<String>) {
    match expr {
        Expr::Ident { name, .. } => {
            if !bound.contains(name) && !free.contains(name) {
                free.push(name.clone());
            }
        }
        Expr::Literal { .. } => {}
        Expr::BinOp { lhs, rhs, .. } => {
            find_free_vars_inner(lhs, bound, free);
            find_free_vars_inner(rhs, bound, free);
        }
        Expr::UnaryOp { operand, .. } => {
            find_free_vars_inner(operand, bound, free);
        }
        Expr::Call { args, .. } => {
            for arg in args {
                find_free_vars_inner(arg, bound, free);
            }
        }
        Expr::Block {
            stmts, tail_expr, ..
        } => {
            find_free_vars_in_stmts(stmts, bound, free);
            if let Some(te) = tail_expr {
                find_free_vars_inner(te, bound, free);
            }
        }
        Expr::If {
            condition,
            then_branch,
            else_branch,
            ..
        } => {
            find_free_vars_inner(condition, bound, free);
            find_free_vars_inner(then_branch, bound, free);
            if let Some(eb) = else_branch {
                find_free_vars_inner(eb, bound, free);
            }
        }
        Expr::ArrayLit { elements, .. } | Expr::TupleLit { elements, .. } => {
            for elem in elements {
                find_free_vars_inner(elem, bound, free);
            }
        }
        Expr::Index { object, index, .. } => {
            find_free_vars_inner(object, bound, free);
            find_free_vars_inner(index, bound, free);
        }
        Expr::FieldAccess { object, .. } | Expr::TupleIndex { object, .. } => {
            find_free_vars_inner(object, bound, free);
        }
        Expr::StructInit { fields, .. } => {
            for (_, val) in fields {
                find_free_vars_inner(val, bound, free);
            }
        }
        Expr::AddrOf { operand, .. }
        | Expr::Deref { operand, .. }
        | Expr::Cast { expr: operand, .. }
        | Expr::Try { operand, .. }
        | Expr::Await { future: operand, .. } => {
            find_free_vars_inner(operand, bound, free);
        }
        Expr::MethodCall { object, args, .. } => {
            find_free_vars_inner(object, bound, free);
            for arg in args {
                find_free_vars_inner(arg, bound, free);
            }
        }
        Expr::Match { subject, arms, .. } => {
            find_free_vars_inner(subject, bound, free);
            for arm in arms {
                // Bindings in pattern become bound within the arm body
                let mut arm_bound: Vec<String> = bound.to_vec();
                if let Pattern::EnumVariant { bindings, .. } = &arm.pattern {
                    arm_bound.extend(bindings.iter().cloned());
                }
                find_free_vars_inner(&arm.body, &arm_bound, free);
            }
        }
        Expr::EnumVariant { args, .. } => {
            for arg in args {
                find_free_vars_inner(arg, bound, free);
            }
        }
        Expr::RangeIndex {
            object, start, end, ..
        } => {
            find_free_vars_inner(object, bound, free);
            find_free_vars_inner(start, bound, free);
            find_free_vars_inner(end, bound, free);
        }
        Expr::Lambda {
            params, body, ..
        } => {
            // Nested lambda params become bound within its body
            let mut inner_bound: Vec<String> = bound.to_vec();
            for p in params {
                inner_bound.push(p.name.clone());
            }
            find_free_vars_inner(body, &inner_bound, free);
        }
    }
}

fn find_free_vars_in_stmts(stmts: &[Stmt], bound: &[String], free: &mut Vec<String>) {
    for stmt in stmts {
        match stmt {
            Stmt::VarDecl { init, .. } | Stmt::ConstDecl { value: init, .. } => {
                find_free_vars_inner(init, bound, free);
            }
            Stmt::Assign { target, value, .. } => {
                find_free_vars_in_assign_target(target, bound, free);
                find_free_vars_inner(value, bound, free);
            }
            Stmt::ExprStmt { expr, .. } => {
                find_free_vars_inner(expr, bound, free);
            }
            Stmt::Return { value, .. } => {
                if let Some(v) = value {
                    find_free_vars_inner(v, bound, free);
                }
            }
            Stmt::While { condition, body, .. } => {
                find_free_vars_inner(condition, bound, free);
                find_free_vars_inner(body, bound, free);
            }
            Stmt::ForRange {
                var,
                start,
                end,
                body,
                ..
            } => {
                find_free_vars_inner(start, bound, free);
                find_free_vars_inner(end, bound, free);
                let mut loop_bound: Vec<String> = bound.to_vec();
                loop_bound.push(var.clone());
                find_free_vars_inner(body, &loop_bound, free);
            }
            Stmt::ForIn {
                var,
                collection,
                body,
                ..
            } => {
                find_free_vars_inner(collection, bound, free);
                let mut loop_bound: Vec<String> = bound.to_vec();
                loop_bound.push(var.clone());
                find_free_vars_inner(body, &loop_bound, free);
            }
            Stmt::TupleDestructure { init, .. } => {
                find_free_vars_inner(init, bound, free);
            }
            Stmt::Defer { body, .. } => {
                find_free_vars_inner(body, bound, free);
            }
            Stmt::WhileLet { expr, body, .. } => {
                find_free_vars_inner(expr, bound, free);
                find_free_vars_inner(body, bound, free);
            }
            Stmt::IfLet {
                expr,
                then_body,
                else_body,
                ..
            } => {
                find_free_vars_inner(expr, bound, free);
                find_free_vars_inner(then_body, bound, free);
                if let Some(eb) = else_body {
                    find_free_vars_inner(eb, bound, free);
                }
            }
            Stmt::Loop { body, .. } => {
                find_free_vars_inner(body, bound, free);
            }
            Stmt::Break { .. } | Stmt::Continue { .. } => {}
        }
    }
}

fn find_free_vars_in_assign_target(
    target: &AssignTarget,
    bound: &[String],
    free: &mut Vec<String>,
) {
    match target {
        AssignTarget::Var(name) => {
            if !bound.contains(name) && !free.contains(name) {
                free.push(name.clone());
            }
        }
        AssignTarget::Index(obj, idx) => {
            find_free_vars_inner(obj, bound, free);
            find_free_vars_inner(idx, bound, free);
        }
        AssignTarget::Field(obj, _) => {
            find_free_vars_inner(obj, bound, free);
        }
        AssignTarget::Deref(obj) => {
            find_free_vars_inner(obj, bound, free);
        }
    }
}

// The compile_expr and compile_expr_as_ptr methods are included below via include.
// Since the content is extracted from mod.rs at build time, we use a placeholder approach.
// Actually, the content follows directly:

impl<'ctx> CodeGen<'ctx> {
    // ------------------------------------------------------------------
    // Compile expressions
    // ------------------------------------------------------------------

    pub(super) fn compile_expr(
        &mut self,
        expr: &Expr,
        function: &FunctionValue<'ctx>,
    ) -> Result<Option<BasicValueEnum<'ctx>>, Vec<CompileError>> {
        match expr {
            // ---- Literals ----
            Expr::Literal { value, .. } => match value {
                LitValue::Int(n) => {
                    let val = self.context.i32_type().const_int(*n as u64, true);
                    Ok(Some(val.into()))
                }
                LitValue::Float(f) => {
                    let val = self.context.f64_type().const_float(*f);
                    Ok(Some(val.into()))
                }
                LitValue::Bool(b) => {
                    let val = self
                        .context
                        .bool_type()
                        .const_int(if *b { 1 } else { 0 }, false);
                    Ok(Some(val.into()))
                }
                LitValue::Str(s) => {
                    // Build a string literal as a {ptr, len} struct value
                    let str_val = self.build_str_literal(s);
                    Ok(Some(str_val))
                }
            },

            // ---- Identifiers ----
            Expr::Ident { name, .. } => {
                if let Some((ptr, ty)) = self.variables.get(name) {
                    let llvm_ty = ny_to_llvm(self.context, ty);
                    let val = self.builder.build_load(llvm_ty, *ptr, name).unwrap();
                    Ok(Some(val))
                } else if let Some((func, _, _)) = self.functions.get(name) {
                    // Function name used as value → return function pointer
                    Ok(Some(func.as_global_value().as_pointer_value().into()))
                } else {
                    Ok(None)
                }
            }

            // ---- Binary operations ----
            Expr::BinOp { op, lhs, rhs, .. } => {
                // Check for operator overloading on struct types
                let lhs_ty = self.infer_expr_type(lhs);
                if let crate::common::NyType::Struct { name, .. } = &lhs_ty {
                    let op_name = match op {
                        BinOp::Add => "add",
                        BinOp::Sub => "sub",
                        BinOp::Mul => "mul",
                        BinOp::Div => "div",
                        BinOp::Eq => "eq",
                        BinOp::Ne => "ne",
                        BinOp::Lt => "lt",
                        BinOp::Gt => "gt",
                        BinOp::Le => "le",
                        BinOp::Ge => "ge",
                        _ => "",
                    };
                    let method = format!("{}_{}", name, op_name);
                    if !op_name.is_empty() {
                        if let Some((func, _, _)) = self.functions.get(&method).cloned() {
                            let lhs_val = self.compile_expr(lhs, function)?.unwrap();
                            let rhs_val = self.compile_expr(rhs, function)?.unwrap();
                            let result = self
                                .builder
                                .build_call(func, &[lhs_val.into(), rhs_val.into()], "op_result")
                                .unwrap()
                                .try_as_basic_value()
                                .basic()
                                .unwrap();
                            return Ok(Some(result));
                        }
                    }
                }

                let lhs_val = self.compile_expr(lhs, function)?.unwrap();
                let rhs_val = self.compile_expr(rhs, function)?.unwrap();
                let result = self.compile_binop(*op, lhs_val, rhs_val)?;
                Ok(Some(result))
            }

            // ---- Unary operations ----
            Expr::UnaryOp { op, operand, .. } => {
                let val = self.compile_expr(operand, function)?.unwrap();
                let result = self.compile_unaryop(*op, val)?;
                Ok(Some(result))
            }

            // ---- Function calls (including print/println builtins) ----
            Expr::Call { callee, args, .. } => {
                // Handle print/println builtins
                if callee == "print" || callee == "println" {
                    self.compile_print_call(callee, args, function)?;
                    return Ok(None);
                }

                // Handle alloc(Type) builtin — returns *Type via malloc
                if callee == "alloc" {
                    // alloc expects exactly 1 argument which evaluates to a size
                    let size_val = self
                        .compile_expr(&args[0], function)?
                        .unwrap()
                        .into_int_value();
                    let size_i64 = self
                        .builder
                        .build_int_z_extend_or_bit_cast(
                            size_val,
                            self.context.i64_type(),
                            "alloc_size",
                        )
                        .unwrap();
                    let malloc_fn = self.get_or_declare_malloc();
                    let ptr = self
                        .builder
                        .build_call(malloc_fn, &[size_i64.into()], "alloc_ptr")
                        .unwrap()
                        .try_as_basic_value()
                        .basic()
                        .unwrap()
                        .into_pointer_value();

                    // Null check: if malloc returned NULL, panic
                    let is_null = self
                        .builder
                        .build_is_null(ptr, "alloc_null")
                        .unwrap();
                    let panic_bb =
                        self.context.append_basic_block(*function, "alloc_panic");
                    let ok_bb =
                        self.context.append_basic_block(*function, "alloc_ok");
                    self.builder
                        .build_conditional_branch(is_null, panic_bb, ok_bb)
                        .unwrap();

                    self.builder.position_at_end(panic_bb);
                    let stderr = self.get_or_declare_stderr();
                    let stderr_ptr = self
                        .builder
                        .build_load(
                            self.context.ptr_type(AddressSpace::default()),
                            stderr.as_pointer_value(),
                            "stderr",
                        )
                        .unwrap();
                    let fprintf_fn = self.get_or_declare_fprintf();
                    let msg = self
                        .builder
                        .build_global_string_ptr(
                            "panic: alloc failed (out of memory)\n",
                            "oom_msg",
                        )
                        .unwrap();
                    self.builder
                        .build_call(
                            fprintf_fn,
                            &[stderr_ptr.into(), msg.as_pointer_value().into()],
                            "",
                        )
                        .unwrap();
                    let trace_print = self.get_or_declare_ny_trace_print();
                    self.builder.build_call(trace_print, &[], "").unwrap();
                    let exit_fn = self.get_or_declare_exit();
                    self.builder
                        .build_call(exit_fn, &[self.context.i32_type().const_int(1, false).into()], "")
                        .unwrap();
                    self.builder.build_unreachable().unwrap();

                    self.builder.position_at_end(ok_bb);
                    return Ok(Some(ptr.into()));
                }

                // Handle free(ptr) builtin
                if callee == "free" {
                    let ptr_val = self.compile_expr(&args[0], function)?.unwrap();
                    let free_fn = self.get_or_declare_free();
                    self.builder
                        .build_call(free_fn, &[ptr_val.into()], "")
                        .unwrap();
                    return Ok(None);
                }

                // Handle fopen(path_str, mode_str) -> FILE*
                if callee == "fopen" {
                    let path_val = self.compile_expr(&args[0], function)?.unwrap();
                    let mode_val = self.compile_expr(&args[1], function)?.unwrap();
                    // Extract ptr from {ptr, len} str structs
                    let path_ptr = self
                        .builder
                        .build_extract_value(path_val.into_struct_value(), 0, "path_ptr")
                        .unwrap();
                    let mode_ptr = self
                        .builder
                        .build_extract_value(mode_val.into_struct_value(), 0, "mode_ptr")
                        .unwrap();
                    let fopen_fn = self.get_or_declare_fopen();
                    let result = self
                        .builder
                        .build_call(fopen_fn, &[path_ptr.into(), mode_ptr.into()], "fp")
                        .unwrap()
                        .try_as_basic_value()
                        .basic()
                        .unwrap();
                    return Ok(Some(result));
                }

                // Handle fclose(fp) -> i32
                if callee == "fclose" {
                    let fp = self.compile_expr(&args[0], function)?.unwrap();
                    let fclose_fn = self.get_or_declare_fclose();
                    let result = self
                        .builder
                        .build_call(fclose_fn, &[fp.into()], "fclose_ret")
                        .unwrap()
                        .try_as_basic_value()
                        .basic()
                        .unwrap();
                    return Ok(Some(result));
                }

                // Handle fwrite_str(fp, str) -> bytes written
                if callee == "fwrite_str" {
                    let fp = self.compile_expr(&args[0], function)?.unwrap();
                    let str_val = self.compile_expr(&args[1], function)?.unwrap();
                    let ptr = self
                        .builder
                        .build_extract_value(str_val.into_struct_value(), 0, "str_ptr")
                        .unwrap();
                    let len = self
                        .builder
                        .build_extract_value(str_val.into_struct_value(), 1, "str_len")
                        .unwrap();
                    let fwrite_fn = self.get_or_declare_fwrite();
                    let one = self.context.i64_type().const_int(1, false);
                    let result = self
                        .builder
                        .build_call(
                            fwrite_fn,
                            &[ptr.into(), one.into(), len.into(), fp.into()],
                            "fwrite_ret",
                        )
                        .unwrap()
                        .try_as_basic_value()
                        .basic()
                        .unwrap();
                    // Cast size_t result to i32
                    let i32_result = self
                        .builder
                        .build_int_truncate(
                            result.into_int_value(),
                            self.context.i32_type(),
                            "fwrite_i32",
                        )
                        .unwrap();
                    return Ok(Some(i32_result.into()));
                }

                // Handle fread_byte(fp) -> i32 (fgetc)
                if callee == "fread_byte" {
                    let fp = self.compile_expr(&args[0], function)?.unwrap();
                    let fgetc_fn = self.get_or_declare_fgetc();
                    let result = self
                        .builder
                        .build_call(fgetc_fn, &[fp.into()], "fgetc_ret")
                        .unwrap()
                        .try_as_basic_value()
                        .basic()
                        .unwrap();
                    return Ok(Some(result));
                }

                // Handle exit(code)
                if callee == "exit" {
                    let code = self.compile_expr(&args[0], function)?.unwrap();
                    let exit_fn = self.get_or_declare_exit();
                    self.builder
                        .build_call(exit_fn, &[code.into()], "")
                        .unwrap();
                    self.builder.build_unreachable().unwrap();
                    return Ok(None);
                }

                // Arena builtins — call C runtime functions
                if callee == "arena_new" {
                    let size_hint = self.compile_expr(&args[0], function)?.unwrap();
                    let size_i64 = self
                        .builder
                        .build_int_s_extend_or_bit_cast(
                            size_hint.into_int_value(),
                            self.context.i64_type(),
                            "arena_size",
                        )
                        .unwrap();
                    let arena_fn = self.get_or_declare_ny_arena_new();
                    let ptr = self
                        .builder
                        .build_call(arena_fn, &[size_i64.into()], "arena")
                        .unwrap()
                        .try_as_basic_value()
                        .basic()
                        .unwrap();
                    return Ok(Some(ptr));
                }
                if callee == "arena_alloc" {
                    let arena = self.compile_expr(&args[0], function)?.unwrap();
                    let size = self.compile_expr(&args[1], function)?.unwrap();
                    let size_i64 = self
                        .builder
                        .build_int_s_extend_or_bit_cast(
                            size.into_int_value(),
                            self.context.i64_type(),
                            "alloc_size",
                        )
                        .unwrap();
                    let alloc_fn = self.get_or_declare_ny_arena_alloc();
                    let ptr = self
                        .builder
                        .build_call(alloc_fn, &[arena.into(), size_i64.into()], "arena_ptr")
                        .unwrap()
                        .try_as_basic_value()
                        .basic()
                        .unwrap();
                    return Ok(Some(ptr));
                }
                if callee == "arena_free" {
                    let arena = self.compile_expr(&args[0], function)?.unwrap();
                    let free_fn = self.get_or_declare_ny_arena_free();
                    self.builder
                        .build_call(free_fn, &[arena.into()], "")
                        .unwrap();
                    return Ok(None);
                }
                if callee == "arena_reset" {
                    let arena = self.compile_expr(&args[0], function)?.unwrap();
                    let reset_fn = self.get_or_declare_ny_arena_reset();
                    self.builder
                        .build_call(reset_fn, &[arena.into()], "")
                        .unwrap();
                    return Ok(None);
                }
                if callee == "arena_bytes_used" {
                    let arena = self.compile_expr(&args[0], function)?.unwrap();
                    let bytes_fn = self.get_or_declare_ny_arena_bytes_used();
                    let result = self
                        .builder
                        .build_call(bytes_fn, &[arena.into()], "arena_used")
                        .unwrap()
                        .try_as_basic_value()
                        .basic()
                        .unwrap();
                    return Ok(Some(result));
                }

                // Handle map_new() → calls ny_map_new() from runtime
                if callee == "map_new" {
                    let map_new_fn = self.get_or_declare_ny_map_new();
                    let ptr = self
                        .builder
                        .build_call(map_new_fn, &[], "map_ptr")
                        .unwrap()
                        .try_as_basic_value()
                        .basic()
                        .unwrap();
                    return Ok(Some(ptr));
                }

                // Handle map_insert(m, key_str, value_i32)
                if callee == "map_insert" {
                    let map_ptr = self.compile_expr(&args[0], function)?.unwrap();
                    let key_val = self
                        .compile_expr(&args[1], function)?
                        .unwrap()
                        .into_struct_value();
                    let key_ptr = self
                        .builder
                        .build_extract_value(key_val, 0, "key_ptr")
                        .unwrap();
                    let key_len = self
                        .builder
                        .build_extract_value(key_val, 1, "key_len")
                        .unwrap();
                    let value = self.compile_expr(&args[2], function)?.unwrap();
                    let value_i64 = self
                        .builder
                        .build_int_s_extend(
                            value.into_int_value(),
                            self.context.i64_type(),
                            "val_i64",
                        )
                        .unwrap();
                    let insert_fn = self.get_or_declare_ny_map_insert();
                    self.builder
                        .build_call(
                            insert_fn,
                            &[
                                map_ptr.into(),
                                key_ptr.into(),
                                key_len.into(),
                                value_i64.into(),
                            ],
                            "",
                        )
                        .unwrap();
                    return Ok(None);
                }

                // Handle map_get(m, key_str) -> i32
                if callee == "map_get" {
                    let map_ptr = self.compile_expr(&args[0], function)?.unwrap();
                    let key_val = self
                        .compile_expr(&args[1], function)?
                        .unwrap()
                        .into_struct_value();
                    let key_ptr = self
                        .builder
                        .build_extract_value(key_val, 0, "key_ptr")
                        .unwrap();
                    let key_len = self
                        .builder
                        .build_extract_value(key_val, 1, "key_len")
                        .unwrap();
                    let get_fn = self.get_or_declare_ny_map_get();
                    let result = self
                        .builder
                        .build_call(
                            get_fn,
                            &[map_ptr.into(), key_ptr.into(), key_len.into()],
                            "map_val",
                        )
                        .unwrap()
                        .try_as_basic_value()
                        .basic()
                        .unwrap();
                    let result_i32 = self
                        .builder
                        .build_int_truncate(
                            result.into_int_value(),
                            self.context.i32_type(),
                            "map_i32",
                        )
                        .unwrap();
                    return Ok(Some(result_i32.into()));
                }

                // Handle map_contains(m, key_str) -> bool
                if callee == "map_contains" {
                    let map_ptr = self.compile_expr(&args[0], function)?.unwrap();
                    let key_val = self
                        .compile_expr(&args[1], function)?
                        .unwrap()
                        .into_struct_value();
                    let key_ptr = self
                        .builder
                        .build_extract_value(key_val, 0, "key_ptr")
                        .unwrap();
                    let key_len = self
                        .builder
                        .build_extract_value(key_val, 1, "key_len")
                        .unwrap();
                    let contains_fn = self.get_or_declare_ny_map_contains();
                    let result = self
                        .builder
                        .build_call(
                            contains_fn,
                            &[map_ptr.into(), key_ptr.into(), key_len.into()],
                            "map_has",
                        )
                        .unwrap()
                        .try_as_basic_value()
                        .basic()
                        .unwrap();
                    let bool_val = self
                        .builder
                        .build_int_compare(
                            IntPredicate::NE,
                            result.into_int_value(),
                            self.context.i32_type().const_zero(),
                            "has_bool",
                        )
                        .unwrap();
                    return Ok(Some(bool_val.into()));
                }

                // Handle map_len(m) -> i64
                if callee == "map_len" {
                    let map_ptr = self.compile_expr(&args[0], function)?.unwrap();
                    let len_fn = self.get_or_declare_ny_map_len();
                    let result = self
                        .builder
                        .build_call(len_fn, &[map_ptr.into()], "map_len")
                        .unwrap()
                        .try_as_basic_value()
                        .basic()
                        .unwrap();
                    return Ok(Some(result));
                }

                // map_remove(m, key_str)
                if callee == "map_remove" {
                    let map_ptr = self.compile_expr(&args[0], function)?.unwrap();
                    let key_val = self
                        .compile_expr(&args[1], function)?
                        .unwrap()
                        .into_struct_value();
                    let key_ptr = self
                        .builder
                        .build_extract_value(key_val, 0, "rm_key_ptr")
                        .unwrap();
                    let key_len = self
                        .builder
                        .build_extract_value(key_val, 1, "rm_key_len")
                        .unwrap();
                    let remove_fn = self.get_or_declare_ny_map_remove();
                    self.builder
                        .build_call(
                            remove_fn,
                            &[map_ptr.into(), key_ptr.into(), key_len.into()],
                            "",
                        )
                        .unwrap();
                    return Ok(None);
                }

                // Tensor builtins
                if callee.starts_with("tensor_") {
                    let c_name = format!("ny_{}", callee);
                    let fn_val = self.get_or_declare_tensor_fn(&c_name);
                    let mut call_args: Vec<inkwell::values::BasicMetadataValueEnum> = Vec::new();
                    for arg in args {
                        let val = self.compile_expr(arg, function)?.unwrap();
                        // Extend i32 to i64 for tensor API (expects i64)
                        if val.is_int_value() {
                            let iv = val.into_int_value();
                            if iv.get_type().get_bit_width() < 64 {
                                let ext = self.builder.build_int_s_extend(
                                    iv, self.context.i64_type(), "t_ext"
                                ).unwrap();
                                call_args.push(ext.into());
                            } else {
                                call_args.push(iv.into());
                            }
                        } else {
                            call_args.push(val.into());
                        }
                    }
                    let result = self.builder.build_call(fn_val, &call_args, "t_r").unwrap();
                    match result.try_as_basic_value().basic() {
                        Some(v) => return Ok(Some(v)),
                        None => return Ok(None),
                    }
                }

                // hmap_new() — create generic HashMap
                if callee == "hmap_new" {
                    // Use 16 bytes as default val_size (covers str {ptr,len}, i64, f64)
                    let val_size = self.context.i64_type().const_int(16, false);
                    let fn_val = self.get_or_declare_ny_hmap_new();
                    let ptr = self.builder.build_call(fn_val, &[val_size.into()], "hmap_ptr").unwrap()
                        .try_as_basic_value().basic().unwrap();
                    return Ok(Some(ptr));
                }

                // === String→String Map (smap) ===
                if callee == "smap_new" {
                    let fn_val = self.get_or_declare_ny_smap_new();
                    let ptr = self.builder.build_call(fn_val, &[], "smap_ptr").unwrap()
                        .try_as_basic_value().basic().unwrap();
                    return Ok(Some(ptr));
                }
                if callee == "smap_insert" {
                    let map_ptr = self.compile_expr(&args[0], function)?.unwrap();
                    let key_val = self.compile_expr(&args[1], function)?.unwrap().into_struct_value();
                    let val_val = self.compile_expr(&args[2], function)?.unwrap().into_struct_value();
                    let kp = self.builder.build_extract_value(key_val, 0, "si_kp").unwrap();
                    let kl = self.builder.build_extract_value(key_val, 1, "si_kl").unwrap();
                    let vp = self.builder.build_extract_value(val_val, 0, "si_vp").unwrap();
                    let vl = self.builder.build_extract_value(val_val, 1, "si_vl").unwrap();
                    let fn_val = self.get_or_declare_ny_smap_insert();
                    self.builder.build_call(fn_val, &[map_ptr.into(), kp.into(), kl.into(), vp.into(), vl.into()], "").unwrap();
                    return Ok(None);
                }
                if callee == "smap_get" {
                    let map_ptr = self.compile_expr(&args[0], function)?.unwrap();
                    let key_val = self.compile_expr(&args[1], function)?.unwrap().into_struct_value();
                    let kp = self.builder.build_extract_value(key_val, 0, "sg_kp").unwrap();
                    let kl = self.builder.build_extract_value(key_val, 1, "sg_kl").unwrap();
                    let i64_ty = self.context.i64_type();
                    let out_len = self.builder.build_alloca(i64_ty, "sg_ol").unwrap();
                    let fn_val = self.get_or_declare_ny_smap_get();
                    let ptr = self.builder.build_call(fn_val, &[map_ptr.into(), kp.into(), kl.into(), out_len.into()], "sg_r").unwrap()
                        .try_as_basic_value().basic().unwrap().into_pointer_value();
                    let len = self.builder.build_load(i64_ty, out_len, "sg_rl").unwrap().into_int_value();
                    let str_ty = str_type(self.context);
                    let result = str_ty.const_zero();
                    let result = self.builder.build_insert_value(result, ptr, 0, "sg_sp").unwrap();
                    let result = self.builder.build_insert_value(result, len, 1, "sg_sl").unwrap();
                    return Ok(Some(result.into_struct_value().into()));
                }
                if callee == "smap_contains" {
                    let map_ptr = self.compile_expr(&args[0], function)?.unwrap();
                    let key_val = self.compile_expr(&args[1], function)?.unwrap().into_struct_value();
                    let kp = self.builder.build_extract_value(key_val, 0, "sc_kp").unwrap();
                    let kl = self.builder.build_extract_value(key_val, 1, "sc_kl").unwrap();
                    let fn_val = self.get_or_declare_ny_smap_contains();
                    let result = self.builder.build_call(fn_val, &[map_ptr.into(), kp.into(), kl.into()], "sc_r").unwrap()
                        .try_as_basic_value().basic().unwrap();
                    return Ok(Some(result));
                }
                if callee == "smap_len" {
                    let map_ptr = self.compile_expr(&args[0], function)?.unwrap();
                    let fn_val = self.get_or_declare_ny_smap_len();
                    let result = self.builder.build_call(fn_val, &[map_ptr.into()], "sl_r").unwrap()
                        .try_as_basic_value().basic().unwrap();
                    return Ok(Some(result));
                }
                if callee == "smap_free" {
                    let map_ptr = self.compile_expr(&args[0], function)?.unwrap();
                    let fn_val = self.get_or_declare_ny_smap_free();
                    self.builder.build_call(fn_val, &[map_ptr.into()], "").unwrap();
                    return Ok(None);
                }

                // map_key_at(m, index) -> str
                if callee == "map_key_at" {
                    let map_ptr = self.compile_expr(&args[0], function)?.unwrap();
                    let idx_val = self.compile_expr(&args[1], function)?.unwrap().into_int_value();
                    let idx_i64 = self
                        .builder
                        .build_int_z_extend_or_bit_cast(
                            idx_val,
                            self.context.i64_type(),
                            "mka_idx",
                        )
                        .unwrap();
                    let i64_ty = self.context.i64_type();
                    let out_len_ptr = self.builder.build_alloca(i64_ty, "mka_olp").unwrap();
                    let fn_val = self.get_or_declare_ny_map_key_at();
                    let key_ptr = self
                        .builder
                        .build_call(
                            fn_val,
                            &[map_ptr.into(), idx_i64.into(), out_len_ptr.into()],
                            "mka_kp",
                        )
                        .unwrap()
                        .try_as_basic_value()
                        .basic()
                        .unwrap()
                        .into_pointer_value();
                    let key_len = self
                        .builder
                        .build_load(i64_ty, out_len_ptr, "mka_kl")
                        .unwrap()
                        .into_int_value();
                    let str_ty = str_type(self.context);
                    let result = str_ty.const_zero();
                    let result = self
                        .builder
                        .build_insert_value(result, key_ptr, 0, "mka_sp")
                        .unwrap();
                    let result = self
                        .builder
                        .build_insert_value(result, key_len, 1, "mka_sl")
                        .unwrap();
                    return Ok(Some(result.into_struct_value().into()));
                }

                // map_free(m)
                if callee == "map_free" {
                    let map_ptr = self.compile_expr(&args[0], function)?.unwrap();
                    let free_fn = self.get_or_declare_ny_map_free();
                    self.builder
                        .build_call(free_fn, &[map_ptr.into()], "")
                        .unwrap();
                    return Ok(None);
                }

                // SIMD builtins
                if callee == "simd_splat_f32x4" {
                    let raw_scalar = self
                        .compile_expr(&args[0], function)?
                        .unwrap()
                        .into_float_value();
                    // Truncate to f32 if the literal is f64
                    let scalar = if raw_scalar.get_type() == self.context.f64_type() {
                        self.builder
                            .build_float_trunc(raw_scalar, self.context.f32_type(), "f32_trunc")
                            .unwrap()
                    } else {
                        raw_scalar
                    };
                    let vec_ty = self.context.f32_type().vec_type(4);
                    let mut vec_val = vec_ty.get_undef();
                    for i in 0..4u32 {
                        let idx = self.context.i32_type().const_int(i as u64, false);
                        vec_val = self
                            .builder
                            .build_insert_element(vec_val, scalar, idx, &format!("splat_{}", i))
                            .unwrap();
                    }
                    return Ok(Some(vec_val.into()));
                }
                if callee == "simd_splat_f32x8" {
                    let raw_scalar = self
                        .compile_expr(&args[0], function)?
                        .unwrap()
                        .into_float_value();
                    let scalar = if raw_scalar.get_type() == self.context.f64_type() {
                        self.builder
                            .build_float_trunc(raw_scalar, self.context.f32_type(), "f32_trunc")
                            .unwrap()
                    } else {
                        raw_scalar
                    };
                    let vec_ty = self.context.f32_type().vec_type(8);
                    let mut vec_val = vec_ty.get_undef();
                    for i in 0..8u32 {
                        let idx = self.context.i32_type().const_int(i as u64, false);
                        vec_val = self
                            .builder
                            .build_insert_element(vec_val, scalar, idx, &format!("splat_{}", i))
                            .unwrap();
                    }
                    return Ok(Some(vec_val.into()));
                }
                if callee == "simd_reduce_add_f32" {
                    let vec_val = self.compile_expr(&args[0], function)?.unwrap();
                    let vec = vec_val.into_vector_value();
                    let lanes = vec.get_type().get_size();
                    let mut sum = self
                        .builder
                        .build_extract_element(vec, self.context.i32_type().const_zero(), "lane_0")
                        .unwrap()
                        .into_float_value();
                    for i in 1..lanes {
                        let idx = self.context.i32_type().const_int(i as u64, false);
                        let lane = self
                            .builder
                            .build_extract_element(vec, idx, &format!("lane_{}", i))
                            .unwrap()
                            .into_float_value();
                        sum = self
                            .builder
                            .build_float_add(sum, lane, &format!("hadd_{}", i))
                            .unwrap();
                    }
                    return Ok(Some(sum.into()));
                }

                // SIMD load: simd_load_f32x4(ptr, offset) → load 4 consecutive f32
                if callee == "simd_load_f32x4" || callee == "simd_load_f32x8" {
                    let lanes: u32 = if callee == "simd_load_f32x4" { 4 } else { 8 };
                    let ptr = self
                        .compile_expr(&args[0], function)?
                        .unwrap()
                        .into_pointer_value();
                    let offset = self
                        .compile_expr(&args[1], function)?
                        .unwrap()
                        .into_int_value();
                    let offset_i64 = self
                        .builder
                        .build_int_s_extend_or_bit_cast(offset, self.context.i64_type(), "off64")
                        .unwrap();
                    // GEP to ptr + offset (byte-level, offset is in elements)
                    let f32_ty = self.context.f32_type();
                    let elem_ptr = unsafe {
                        self.builder
                            .build_in_bounds_gep(f32_ty, ptr, &[offset_i64], "simd_ptr")
                            .unwrap()
                    };
                    // Bitcast to vector pointer and load
                    let vec_ty = f32_ty.vec_type(lanes);
                    let vec_val = self
                        .builder
                        .build_load(vec_ty, elem_ptr, "simd_load")
                        .unwrap();
                    return Ok(Some(vec_val));
                }

                // SIMD store: simd_store_f32x4(ptr, offset, vec)
                if callee == "simd_store_f32x4" || callee == "simd_store_f32x8" {
                    let ptr = self
                        .compile_expr(&args[0], function)?
                        .unwrap()
                        .into_pointer_value();
                    let offset = self
                        .compile_expr(&args[1], function)?
                        .unwrap()
                        .into_int_value();
                    let vec_val = self.compile_expr(&args[2], function)?.unwrap();
                    let offset_i64 = self
                        .builder
                        .build_int_s_extend_or_bit_cast(offset, self.context.i64_type(), "off64")
                        .unwrap();
                    let f32_ty = self.context.f32_type();
                    let elem_ptr = unsafe {
                        self.builder
                            .build_in_bounds_gep(f32_ty, ptr, &[offset_i64], "store_ptr")
                            .unwrap()
                    };
                    self.builder.build_store(elem_ptr, vec_val).unwrap();
                    return Ok(None);
                }

                // to_str(val) → converts any type to str via snprintf
                if callee == "to_str" {
                    let arg_ty = self.infer_expr_type(&args[0]);
                    let val = self.compile_expr(&args[0], function)?.unwrap();

                    let buf_size = self.context.i64_type().const_int(64, false);
                    let malloc_fn = self.get_or_declare_malloc();
                    let buf_ptr = self
                        .builder
                        .build_call(malloc_fn, &[buf_size.into()], "ts_buf")
                        .unwrap()
                        .try_as_basic_value()
                        .basic()
                        .unwrap()
                        .into_pointer_value();
                    let snprintf_fn = self.get_or_declare_snprintf();

                    let fmt_str = match &arg_ty {
                        NyType::I32 => "%d",
                        NyType::I64 => "%ld",
                        t if t.is_integer() => "%ld",
                        NyType::F32 | NyType::F64 => "%f",
                        NyType::Bool => "%s",
                        NyType::Str => "%s", // identity for strings
                        _ => "%d",
                    };
                    let fmt = self
                        .builder
                        .build_global_string_ptr(fmt_str, "ts_fmt")
                        .unwrap();

                    let print_val: BasicValueEnum = if arg_ty == NyType::Bool {
                        let b = val.into_int_value();
                        let ts = self
                            .builder
                            .build_global_string_ptr("true", "ts_t")
                            .unwrap();
                        let fs = self
                            .builder
                            .build_global_string_ptr("false", "ts_f")
                            .unwrap();
                        self.builder
                            .build_select(b, ts.as_pointer_value(), fs.as_pointer_value(), "ts_sel")
                            .unwrap()
                    } else if arg_ty == NyType::Str {
                        let sv = val.into_struct_value();
                        self.builder.build_extract_value(sv, 0, "ts_ptr").unwrap()
                    } else if arg_ty.is_integer() && arg_ty != NyType::I32 {
                        let ext = self
                            .builder
                            .build_int_s_extend(
                                val.into_int_value(),
                                self.context.i64_type(),
                                "ts_ext",
                            )
                            .unwrap();
                        ext.into()
                    } else {
                        val
                    };

                    self.builder
                        .build_call(
                            snprintf_fn,
                            &[
                                buf_ptr.into(),
                                buf_size.into(),
                                fmt.as_pointer_value().into(),
                                print_val.into(),
                            ],
                            "",
                        )
                        .unwrap();

                    let strlen_fn = self.get_or_declare_strlen();
                    let len = self
                        .builder
                        .build_call(strlen_fn, &[buf_ptr.into()], "ts_len")
                        .unwrap()
                        .try_as_basic_value()
                        .basic()
                        .unwrap()
                        .into_int_value();

                    let str_ty = str_type(self.context);
                    let str_val = str_ty.const_zero();
                    let str_val = self
                        .builder
                        .build_insert_value(str_val, buf_ptr, 0, "ts_p")
                        .unwrap();
                    let str_val = self
                        .builder
                        .build_insert_value(str_val, len, 1, "ts_l")
                        .unwrap();
                    return Ok(Some(str_val.into_struct_value().into()));
                }

                // Channel builtins
                if callee == "channel_new" {
                    let cap = self.compile_expr(&args[0], function)?.unwrap();
                    let fn_decl = self.get_or_declare_c_fn(
                        "ny_channel_new",
                        self.context
                            .ptr_type(AddressSpace::default())
                            .fn_type(&[self.context.i32_type().into()], false),
                    );
                    let ptr = self
                        .builder
                        .build_call(fn_decl, &[cap.into()], "ch")
                        .unwrap()
                        .try_as_basic_value()
                        .basic()
                        .unwrap();
                    return Ok(Some(ptr));
                }
                if callee == "channel_send" {
                    let ch = self.compile_expr(&args[0], function)?.unwrap();
                    let val = self.compile_expr(&args[1], function)?.unwrap();
                    let fn_decl = self.get_or_declare_c_fn(
                        "ny_channel_send",
                        self.context.void_type().fn_type(
                            &[
                                self.context.ptr_type(AddressSpace::default()).into(),
                                self.context.i32_type().into(),
                            ],
                            false,
                        ),
                    );
                    self.builder
                        .build_call(fn_decl, &[ch.into(), val.into()], "")
                        .unwrap();
                    return Ok(None);
                }
                if callee == "channel_recv" {
                    let ch = self.compile_expr(&args[0], function)?.unwrap();
                    let fn_decl = self.get_or_declare_c_fn(
                        "ny_channel_recv",
                        self.context.i32_type().fn_type(
                            &[self.context.ptr_type(AddressSpace::default()).into()],
                            false,
                        ),
                    );
                    let val = self
                        .builder
                        .build_call(fn_decl, &[ch.into()], "recv")
                        .unwrap()
                        .try_as_basic_value()
                        .basic()
                        .unwrap();
                    return Ok(Some(val));
                }
                if callee == "channel_close" {
                    let ch = self.compile_expr(&args[0], function)?.unwrap();
                    let fn_decl = self.get_or_declare_c_fn(
                        "ny_channel_close",
                        self.context.void_type().fn_type(
                            &[self.context.ptr_type(AddressSpace::default()).into()],
                            false,
                        ),
                    );
                    self.builder.build_call(fn_decl, &[ch.into()], "").unwrap();
                    return Ok(None);
                }

                // Pool builtins
                if callee == "pool_new" {
                    let n = self.compile_expr(&args[0], function)?.unwrap();
                    let fn_decl = self.get_or_declare_c_fn(
                        "ny_pool_new",
                        self.context
                            .ptr_type(AddressSpace::default())
                            .fn_type(&[self.context.i32_type().into()], false),
                    );
                    let ptr = self
                        .builder
                        .build_call(fn_decl, &[n.into()], "pool")
                        .unwrap()
                        .try_as_basic_value()
                        .basic()
                        .unwrap();
                    return Ok(Some(ptr));
                }
                if callee == "pool_submit" {
                    let pool = self.compile_expr(&args[0], function)?.unwrap();
                    let fn_ptr = self.compile_expr(&args[1], function)?.unwrap();
                    let fn_decl = self.get_or_declare_c_fn(
                        "ny_pool_submit",
                        self.context.void_type().fn_type(
                            &[
                                self.context.ptr_type(AddressSpace::default()).into(),
                                self.context.ptr_type(AddressSpace::default()).into(),
                            ],
                            false,
                        ),
                    );
                    self.builder
                        .build_call(fn_decl, &[pool.into(), fn_ptr.into()], "")
                        .unwrap();
                    return Ok(None);
                }
                if callee == "pool_wait" {
                    let pool = self.compile_expr(&args[0], function)?.unwrap();
                    let fn_decl = self.get_or_declare_c_fn(
                        "ny_pool_wait",
                        self.context.void_type().fn_type(
                            &[self.context.ptr_type(AddressSpace::default()).into()],
                            false,
                        ),
                    );
                    self.builder
                        .build_call(fn_decl, &[pool.into()], "")
                        .unwrap();
                    return Ok(None);
                }
                if callee == "pool_free" {
                    let pool = self.compile_expr(&args[0], function)?.unwrap();
                    let fn_decl = self.get_or_declare_c_fn(
                        "ny_pool_free",
                        self.context.void_type().fn_type(
                            &[self.context.ptr_type(AddressSpace::default()).into()],
                            false,
                        ),
                    );
                    self.builder
                        .build_call(fn_decl, &[pool.into()], "")
                        .unwrap();
                    return Ok(None);
                }

                // Parallel iterator builtins
                if callee == "par_map" {
                    let data = self.compile_expr(&args[0], function)?.unwrap();
                    let n = self.compile_expr(&args[1], function)?.unwrap();
                    let result = self.compile_expr(&args[2], function)?.unwrap();
                    let map_fn = self.compile_expr(&args[3], function)?.unwrap();
                    let pool = self.compile_expr(&args[4], function)?.unwrap();
                    let ptr_ty = self.context.ptr_type(AddressSpace::default());
                    let i32_ty = self.context.i32_type();
                    let fn_decl = self.get_or_declare_c_fn(
                        "ny_par_map",
                        self.context.void_type().fn_type(
                            &[
                                ptr_ty.into(),
                                i32_ty.into(),
                                ptr_ty.into(),
                                ptr_ty.into(),
                                ptr_ty.into(),
                            ],
                            false,
                        ),
                    );
                    self.builder
                        .build_call(
                            fn_decl,
                            &[
                                data.into(),
                                n.into(),
                                result.into(),
                                map_fn.into(),
                                pool.into(),
                            ],
                            "",
                        )
                        .unwrap();
                    return Ok(None);
                }
                if callee == "par_reduce" {
                    let data = self.compile_expr(&args[0], function)?.unwrap();
                    let n = self.compile_expr(&args[1], function)?.unwrap();
                    let init = self.compile_expr(&args[2], function)?.unwrap();
                    let reduce_fn = self.compile_expr(&args[3], function)?.unwrap();
                    let pool = self.compile_expr(&args[4], function)?.unwrap();
                    let ptr_ty = self.context.ptr_type(AddressSpace::default());
                    let i32_ty = self.context.i32_type();
                    let fn_decl = self.get_or_declare_c_fn(
                        "ny_par_reduce",
                        i32_ty.fn_type(
                            &[
                                ptr_ty.into(),
                                i32_ty.into(),
                                i32_ty.into(),
                                ptr_ty.into(),
                                ptr_ty.into(),
                            ],
                            false,
                        ),
                    );
                    let val = self
                        .builder
                        .build_call(
                            fn_decl,
                            &[
                                data.into(),
                                n.into(),
                                init.into(),
                                reduce_fn.into(),
                                pool.into(),
                            ],
                            "par_red",
                        )
                        .unwrap()
                        .try_as_basic_value()
                        .basic()
                        .unwrap();
                    return Ok(Some(val));
                }

                // Thread builtins
                if callee == "thread_spawn" {
                    // thread_spawn(fn_ptr) or thread_spawn(fn_ptr, arg)
                    let fn_ptr = self.compile_expr(&args[0], function)?.unwrap();
                    let pthread_create = self.get_or_declare_pthread_create();
                    let handle_alloca = self
                        .builder
                        .build_alloca(self.context.i64_type(), "thread_handle")
                        .unwrap();
                    let null = self.context.ptr_type(AddressSpace::default()).const_null();
                    let thread_arg = if args.len() > 1 {
                        self.compile_expr(&args[1], function)?.unwrap()
                    } else {
                        null.into()
                    };
                    self.builder
                        .build_call(
                            pthread_create,
                            &[
                                handle_alloca.into(),
                                null.into(),
                                fn_ptr.into(),
                                thread_arg.into(),
                            ],
                            "spawn",
                        )
                        .unwrap();
                    let handle = self
                        .builder
                        .build_load(self.context.i64_type(), handle_alloca, "tid")
                        .unwrap();
                    return Ok(Some(handle));
                }

                if callee == "thread_join" {
                    let handle = self.compile_expr(&args[0], function)?.unwrap();
                    let pthread_join = self.get_or_declare_pthread_join();
                    let null = self.context.ptr_type(AddressSpace::default()).const_null();
                    self.builder
                        .build_call(pthread_join, &[handle.into(), null.into()], "join")
                        .unwrap();
                    return Ok(None);
                }

                // Handle vec_new() — creates empty Vec with correct elem_size
                if callee == "vec_new" {
                    let initial_cap: u64 = 8;
                    // Default elem_size 8 (overridden in VarDecl when type annotation present)
                    let elem_size_val = self.context.i64_type().const_int(8, false);
                    let alloc_size = self
                        .builder
                        .build_int_mul(
                            self.context.i64_type().const_int(initial_cap, false),
                            elem_size_val,
                            "vec_alloc_size",
                        )
                        .unwrap();
                    let malloc_fn = self.get_or_declare_malloc();
                    let data_ptr = self
                        .builder
                        .build_call(malloc_fn, &[alloc_size.into()], "vec_data")
                        .unwrap()
                        .try_as_basic_value()
                        .basic()
                        .unwrap()
                        .into_pointer_value();
                    let vec_ty = ny_to_llvm(self.context, &NyType::Vec(Box::new(NyType::I32)));
                    let vec_val = vec_ty.into_struct_type().const_zero();
                    let vec_val = self
                        .builder
                        .build_insert_value(vec_val, data_ptr, 0, "vec_p")
                        .unwrap();
                    let vec_val = self
                        .builder
                        .build_insert_value(
                            vec_val,
                            self.context.i64_type().const_zero(),
                            1,
                            "vec_l",
                        )
                        .unwrap();
                    let vec_val = self
                        .builder
                        .build_insert_value(
                            vec_val,
                            self.context.i64_type().const_int(initial_cap, false),
                            2,
                            "vec_c",
                        )
                        .unwrap();
                    let vec_val = self
                        .builder
                        .build_insert_value(vec_val, elem_size_val, 3, "vec_es")
                        .unwrap();
                    return Ok(Some(vec_val.into_struct_value().into()));
                }

                // Handle read_line() — reads a line from stdin using fgets
                if callee == "read_line" {
                    // Allocate a 1024-byte buffer
                    let buf_size = self.context.i64_type().const_int(1024, false);
                    let malloc_fn = self.get_or_declare_malloc();
                    let buf_ptr = self
                        .builder
                        .build_call(malloc_fn, &[buf_size.into()], "line_buf")
                        .unwrap()
                        .try_as_basic_value()
                        .basic()
                        .unwrap()
                        .into_pointer_value();

                    // Call fgets(buf, 1024, stdin)
                    let fgets_fn = self.get_or_declare_fgets();
                    let stdin_fn = self.get_or_declare_stdin();
                    let stdin_val = self
                        .builder
                        .build_load(
                            self.context.ptr_type(AddressSpace::default()),
                            stdin_fn.as_pointer_value(),
                            "stdin_val",
                        )
                        .unwrap();
                    let size_i32 = self.context.i32_type().const_int(1024, false);
                    self.builder
                        .build_call(
                            fgets_fn,
                            &[buf_ptr.into(), size_i32.into(), stdin_val.into()],
                            "fgets_ret",
                        )
                        .unwrap();

                    // Compute length with strlen
                    let strlen_fn = self.get_or_declare_strlen();
                    let len = self
                        .builder
                        .build_call(strlen_fn, &[buf_ptr.into()], "line_len")
                        .unwrap()
                        .try_as_basic_value()
                        .basic()
                        .unwrap()
                        .into_int_value();

                    // Strip trailing newline: if buf[len-1] == '\n', len--
                    let one = self.context.i64_type().const_int(1, false);
                    let len_minus_1 = self.builder.build_int_sub(len, one, "len_m1").unwrap();
                    let last_char_ptr = unsafe {
                        self.builder
                            .build_in_bounds_gep(
                                self.context.i8_type(),
                                buf_ptr,
                                &[len_minus_1],
                                "last_ptr",
                            )
                            .unwrap()
                    };
                    let last_char = self
                        .builder
                        .build_load(self.context.i8_type(), last_char_ptr, "last_char")
                        .unwrap()
                        .into_int_value();
                    let newline = self.context.i8_type().const_int(10, false); // '\n'
                    let is_newline = self
                        .builder
                        .build_int_compare(IntPredicate::EQ, last_char, newline, "is_nl")
                        .unwrap();
                    let final_len = self
                        .builder
                        .build_select(is_newline, len_minus_1, len, "final_len")
                        .unwrap()
                        .into_int_value();

                    // Build {ptr, len} str
                    let str_ty = str_type(self.context);
                    let str_val = str_ty.const_zero();
                    let str_val = self
                        .builder
                        .build_insert_value(str_val, buf_ptr, 0, "rl_ptr")
                        .unwrap();
                    let str_val = self
                        .builder
                        .build_insert_value(str_val, final_len, 1, "rl_len")
                        .unwrap();
                    return Ok(Some(str_val.into_struct_value().into()));
                }

                // Handle str_to_int(s) — wraps atoi
                if callee == "str_to_int" {
                    let str_val = self
                        .compile_expr(&args[0], function)?
                        .unwrap()
                        .into_struct_value();
                    let ptr = self
                        .builder
                        .build_extract_value(str_val, 0, "s2i_ptr")
                        .unwrap();
                    let atoi_fn = self.get_or_declare_atoi();
                    let result = self
                        .builder
                        .build_call(atoi_fn, &[ptr.into()], "atoi_ret")
                        .unwrap()
                        .try_as_basic_value()
                        .basic()
                        .unwrap();
                    return Ok(Some(result));
                }

                // Handle int_to_str(n) — wraps snprintf
                if callee == "int_to_str" {
                    let int_val = self.compile_expr(&args[0], function)?.unwrap();

                    // Allocate buffer for the string representation
                    let buf_size = self.context.i64_type().const_int(32, false);
                    let malloc_fn = self.get_or_declare_malloc();
                    let buf_ptr = self
                        .builder
                        .build_call(malloc_fn, &[buf_size.into()], "i2s_buf")
                        .unwrap()
                        .try_as_basic_value()
                        .basic()
                        .unwrap()
                        .into_pointer_value();

                    // snprintf(buf, 32, "%d", val)
                    let snprintf_fn = self.get_or_declare_snprintf();
                    let fmt = self
                        .builder
                        .build_global_string_ptr("%d", "i2s_fmt")
                        .unwrap();
                    let size_i64 = self.context.i64_type().const_int(32, false);
                    self.builder
                        .build_call(
                            snprintf_fn,
                            &[
                                buf_ptr.into(),
                                size_i64.into(),
                                fmt.as_pointer_value().into(),
                                int_val.into(),
                            ],
                            "snprintf_ret",
                        )
                        .unwrap();

                    // Get length with strlen
                    let strlen_fn = self.get_or_declare_strlen();
                    let len = self
                        .builder
                        .build_call(strlen_fn, &[buf_ptr.into()], "i2s_len")
                        .unwrap()
                        .try_as_basic_value()
                        .basic()
                        .unwrap()
                        .into_int_value();

                    // Build {ptr, len} str
                    let str_ty = str_type(self.context);
                    let str_val = str_ty.const_zero();
                    let str_val = self
                        .builder
                        .build_insert_value(str_val, buf_ptr, 0, "i2s_p")
                        .unwrap();
                    let str_val = self
                        .builder
                        .build_insert_value(str_val, len, 1, "i2s_l")
                        .unwrap();
                    return Ok(Some(str_val.into_struct_value().into()));
                }

                // Handle sleep_ms(ms) — wraps usleep(ms * 1000)
                if callee == "sleep_ms" {
                    let ms_val = self
                        .compile_expr(&args[0], function)?
                        .unwrap()
                        .into_int_value();
                    let us_val = self
                        .builder
                        .build_int_mul(ms_val, self.context.i32_type().const_int(1000, false), "us")
                        .unwrap();
                    let usleep_fn = self.get_or_declare_usleep();
                    self.builder
                        .build_call(usleep_fn, &[us_val.into()], "")
                        .unwrap();
                    return Ok(None);
                }

                // Handle str_split_count(str, delim) -> i32
                if callee == "str_split_count" {
                    let str_val = self
                        .compile_expr(&args[0], function)?
                        .unwrap()
                        .into_struct_value();
                    let delim_val = self
                        .compile_expr(&args[1], function)?
                        .unwrap()
                        .into_struct_value();

                    let hay_ptr = self
                        .builder
                        .build_extract_value(str_val, 0, "sp_hay_ptr")
                        .unwrap();
                    let hay_len = self
                        .builder
                        .build_extract_value(str_val, 1, "sp_hay_len")
                        .unwrap();
                    let delim_ptr = self
                        .builder
                        .build_extract_value(delim_val, 0, "sp_dlm_ptr")
                        .unwrap();
                    let delim_len = self
                        .builder
                        .build_extract_value(delim_val, 1, "sp_dlm_len")
                        .unwrap();

                    let i64_ty = self.context.i64_type();
                    let count_ptr = self
                        .builder
                        .build_alloca(i64_ty, "sp_count")
                        .unwrap();

                    let split_fn = self.get_or_declare_ny_str_split();
                    let _parts = self
                        .builder
                        .build_call(
                            split_fn,
                            &[
                                hay_ptr.into(),
                                hay_len.into(),
                                delim_ptr.into(),
                                delim_len.into(),
                                count_ptr.into(),
                            ],
                            "sp_parts",
                        )
                        .unwrap();

                    let count = self
                        .builder
                        .build_load(i64_ty, count_ptr, "sp_count_val")
                        .unwrap()
                        .into_int_value();
                    let count_i32 = self
                        .builder
                        .build_int_truncate(count, self.context.i32_type(), "sp_count_i32")
                        .unwrap();
                    // Free the allocated parts array
                    let parts_ptr = _parts
                        .try_as_basic_value()
                        .basic()
                        .unwrap()
                        .into_pointer_value();
                    let free_fn = self.get_or_declare_free();
                    self.builder
                        .build_call(free_fn, &[parts_ptr.into()], "")
                        .unwrap();

                    return Ok(Some(count_i32.into()));
                }

                // Handle str_split_get(str, delim, index) -> str
                if callee == "str_split_get" {
                    let str_val = self
                        .compile_expr(&args[0], function)?
                        .unwrap()
                        .into_struct_value();
                    let delim_val = self
                        .compile_expr(&args[1], function)?
                        .unwrap()
                        .into_struct_value();
                    let idx_val = self
                        .compile_expr(&args[2], function)?
                        .unwrap()
                        .into_int_value();

                    let hay_ptr = self
                        .builder
                        .build_extract_value(str_val, 0, "sg_hay_ptr")
                        .unwrap();
                    let hay_len = self
                        .builder
                        .build_extract_value(str_val, 1, "sg_hay_len")
                        .unwrap();
                    let delim_ptr = self
                        .builder
                        .build_extract_value(delim_val, 0, "sg_dlm_ptr")
                        .unwrap();
                    let delim_len = self
                        .builder
                        .build_extract_value(delim_val, 1, "sg_dlm_len")
                        .unwrap();

                    let i64_ty = self.context.i64_type();
                    let count_ptr = self
                        .builder
                        .build_alloca(i64_ty, "sg_count")
                        .unwrap();

                    let split_fn = self.get_or_declare_ny_str_split();
                    let parts = self
                        .builder
                        .build_call(
                            split_fn,
                            &[
                                hay_ptr.into(),
                                hay_len.into(),
                                delim_ptr.into(),
                                delim_len.into(),
                                count_ptr.into(),
                            ],
                            "sg_parts",
                        )
                        .unwrap()
                        .try_as_basic_value()
                        .basic()
                        .unwrap()
                        .into_pointer_value();

                    // Each NyStrSlice is {ptr: *u8, len: i64} = 16 bytes
                    // Access parts[idx]: GEP with stride of 16 bytes
                    let idx_i64 = self
                        .builder
                        .build_int_z_extend_or_bit_cast(idx_val, i64_ty, "sg_idx64")
                        .unwrap();

                    // Use i8 GEP with stride 16
                    let sixteen = i64_ty.const_int(16, false);
                    let byte_offset = self
                        .builder
                        .build_int_mul(idx_i64, sixteen, "sg_off")
                        .unwrap();
                    let i8_ty = self.context.i8_type();
                    let elem_ptr = unsafe {
                        self.builder
                            .build_in_bounds_gep(i8_ty, parts, &[byte_offset], "sg_elem")
                            .unwrap()
                    };

                    // Load ptr (first 8 bytes)
                    let ptr_ty = self.context.ptr_type(inkwell::AddressSpace::default());
                    let s_ptr = self
                        .builder
                        .build_load(ptr_ty, elem_ptr, "sg_s_ptr")
                        .unwrap()
                        .into_pointer_value();

                    // Load len (next 8 bytes, offset +8)
                    let eight = i64_ty.const_int(8, false);
                    let len_byte_off = self
                        .builder
                        .build_int_add(byte_offset, eight, "sg_len_off")
                        .unwrap();
                    let len_ptr = unsafe {
                        self.builder
                            .build_in_bounds_gep(i8_ty, parts, &[len_byte_off], "sg_len_p")
                            .unwrap()
                    };
                    let s_len = self
                        .builder
                        .build_load(i64_ty, len_ptr, "sg_s_len")
                        .unwrap()
                        .into_int_value();

                    // Free parts array
                    let free_fn = self.get_or_declare_free();
                    self.builder
                        .build_call(free_fn, &[parts.into()], "")
                        .unwrap();

                    // Build {ptr, len} str result
                    let str_ty = str_type(self.context);
                    let result = str_ty.const_zero();
                    let result = self
                        .builder
                        .build_insert_value(result, s_ptr, 0, "sg_rp")
                        .unwrap();
                    let result = self
                        .builder
                        .build_insert_value(result, s_len, 1, "sg_rl")
                        .unwrap();
                    return Ok(Some(result.into_struct_value().into()));
                }

                // JSON builtins
                if callee == "json_parse" {
                    let str_val = self.compile_expr(&args[0], function)?.unwrap().into_struct_value();
                    let ptr = self.builder.build_extract_value(str_val, 0, "jp_p").unwrap();
                    let len = self.builder.build_extract_value(str_val, 1, "jp_l").unwrap();
                    let jp_fn = self.get_or_declare_ny_json_parse();
                    let result = self.builder.build_call(jp_fn, &[ptr.into(), len.into()], "jp_r").unwrap()
                        .try_as_basic_value().basic().unwrap();
                    return Ok(Some(result));
                }
                if callee == "json_type" {
                    let obj = self.compile_expr(&args[0], function)?.unwrap();
                    let fn_val = self.get_or_declare_ny_json_type();
                    let result = self.builder.build_call(fn_val, &[obj.into()], "jt_r").unwrap()
                        .try_as_basic_value().basic().unwrap();
                    return Ok(Some(result));
                }
                if callee == "json_get_int" {
                    let obj = self.compile_expr(&args[0], function)?.unwrap();
                    let key_val = self.compile_expr(&args[1], function)?.unwrap().into_struct_value();
                    let kp = self.builder.build_extract_value(key_val, 0, "jgi_kp").unwrap();
                    let kl = self.builder.build_extract_value(key_val, 1, "jgi_kl").unwrap();
                    let fn_val = self.get_or_declare_ny_json_get_int();
                    let result = self.builder.build_call(fn_val, &[obj.into(), kp.into(), kl.into()], "jgi_r").unwrap()
                        .try_as_basic_value().basic().unwrap();
                    // Truncate i64 to i32
                    let i32_val = self.builder.build_int_truncate(result.into_int_value(), self.context.i32_type(), "jgi_i32").unwrap();
                    return Ok(Some(i32_val.into()));
                }
                if callee == "json_get_float" {
                    let obj = self.compile_expr(&args[0], function)?.unwrap();
                    let key_val = self.compile_expr(&args[1], function)?.unwrap().into_struct_value();
                    let kp = self.builder.build_extract_value(key_val, 0, "jgf_kp").unwrap();
                    let kl = self.builder.build_extract_value(key_val, 1, "jgf_kl").unwrap();
                    let fn_val = self.get_or_declare_ny_json_get_float();
                    let result = self.builder.build_call(fn_val, &[obj.into(), kp.into(), kl.into()], "jgf_r").unwrap()
                        .try_as_basic_value().basic().unwrap();
                    return Ok(Some(result));
                }
                if callee == "json_get_str" {
                    let obj = self.compile_expr(&args[0], function)?.unwrap();
                    let key_val = self.compile_expr(&args[1], function)?.unwrap().into_struct_value();
                    let kp = self.builder.build_extract_value(key_val, 0, "jgs_kp").unwrap();
                    let kl = self.builder.build_extract_value(key_val, 1, "jgs_kl").unwrap();
                    let i64_ty = self.context.i64_type();
                    let out_len_ptr = self.builder.build_alloca(i64_ty, "jgs_olp").unwrap();
                    let fn_val = self.get_or_declare_ny_json_get_str();
                    let buf = self.builder.build_call(fn_val, &[obj.into(), kp.into(), kl.into(), out_len_ptr.into()], "jgs_r").unwrap()
                        .try_as_basic_value().basic().unwrap().into_pointer_value();
                    let out_len = self.builder.build_load(i64_ty, out_len_ptr, "jgs_len").unwrap().into_int_value();
                    let str_ty = str_type(self.context);
                    let result = str_ty.const_zero();
                    let result = self.builder.build_insert_value(result, buf, 0, "jgs_sp").unwrap();
                    let result = self.builder.build_insert_value(result, out_len, 1, "jgs_sl").unwrap();
                    return Ok(Some(result.into_struct_value().into()));
                }
                if callee == "json_get_bool" {
                    let obj = self.compile_expr(&args[0], function)?.unwrap();
                    let key_val = self.compile_expr(&args[1], function)?.unwrap().into_struct_value();
                    let kp = self.builder.build_extract_value(key_val, 0, "jgb_kp").unwrap();
                    let kl = self.builder.build_extract_value(key_val, 1, "jgb_kl").unwrap();
                    let fn_val = self.get_or_declare_ny_json_get_bool();
                    let result = self.builder.build_call(fn_val, &[obj.into(), kp.into(), kl.into()], "jgb_r").unwrap()
                        .try_as_basic_value().basic().unwrap();
                    return Ok(Some(result));
                }
                if callee == "json_len" {
                    let obj = self.compile_expr(&args[0], function)?.unwrap();
                    let fn_val = self.get_or_declare_ny_json_len();
                    let result = self.builder.build_call(fn_val, &[obj.into()], "jl_r").unwrap()
                        .try_as_basic_value().basic().unwrap();
                    let i32_val = self.builder.build_int_truncate(result.into_int_value(), self.context.i32_type(), "jl_i32").unwrap();
                    return Ok(Some(i32_val.into()));
                }
                if callee == "json_arr_get" {
                    let arr = self.compile_expr(&args[0], function)?.unwrap();
                    let idx = self.compile_expr(&args[1], function)?.unwrap().into_int_value();
                    let idx_i64 = self.builder.build_int_z_extend_or_bit_cast(idx, self.context.i64_type(), "ja_idx").unwrap();
                    let fn_val = self.get_or_declare_ny_json_arr_get();
                    let result = self.builder.build_call(fn_val, &[arr.into(), idx_i64.into()], "ja_r").unwrap()
                        .try_as_basic_value().basic().unwrap();
                    return Ok(Some(result));
                }
                if callee == "json_free" {
                    let obj = self.compile_expr(&args[0], function)?.unwrap();
                    let fn_val = self.get_or_declare_ny_json_free();
                    self.builder.build_call(fn_val, &[obj.into()], "").unwrap();
                    return Ok(None);
                }

                // remove_file(path: str) -> i32
                if callee == "remove_file" {
                    let path_val = self
                        .compile_expr(&args[0], function)?
                        .unwrap()
                        .into_struct_value();
                    let pp = self.builder.build_extract_value(path_val, 0, "rmf_pp").unwrap();
                    let pl = self.builder.build_extract_value(path_val, 1, "rmf_pl").unwrap();
                    let rmf_fn = self.get_or_declare_ny_remove_file();
                    let result = self
                        .builder
                        .build_call(rmf_fn, &[pp.into(), pl.into()], "rmf_r")
                        .unwrap()
                        .try_as_basic_value()
                        .basic()
                        .unwrap();
                    return Ok(Some(result));
                }

                // read_file(path: str) -> str
                if callee == "read_file" {
                    let path_val = self
                        .compile_expr(&args[0], function)?
                        .unwrap()
                        .into_struct_value();
                    let path_ptr = self
                        .builder
                        .build_extract_value(path_val, 0, "rf_pp")
                        .unwrap();
                    let path_len = self
                        .builder
                        .build_extract_value(path_val, 1, "rf_pl")
                        .unwrap();
                    let i64_ty = self.context.i64_type();
                    let out_len_ptr = self.builder.build_alloca(i64_ty, "rf_olp").unwrap();
                    let rf_fn = self.get_or_declare_ny_read_file();
                    let buf = self
                        .builder
                        .build_call(
                            rf_fn,
                            &[path_ptr.into(), path_len.into(), out_len_ptr.into()],
                            "rf_buf",
                        )
                        .unwrap()
                        .try_as_basic_value()
                        .basic()
                        .unwrap()
                        .into_pointer_value();
                    let out_len = self
                        .builder
                        .build_load(i64_ty, out_len_ptr, "rf_len")
                        .unwrap()
                        .into_int_value();
                    let str_ty = str_type(self.context);
                    let result = str_ty.const_zero();
                    let result = self
                        .builder
                        .build_insert_value(result, buf, 0, "rf_sp")
                        .unwrap();
                    let result = self
                        .builder
                        .build_insert_value(result, out_len, 1, "rf_sl")
                        .unwrap();
                    return Ok(Some(result.into_struct_value().into()));
                }

                // write_file(path: str, content: str) -> i32
                if callee == "write_file" {
                    let path_val = self
                        .compile_expr(&args[0], function)?
                        .unwrap()
                        .into_struct_value();
                    let data_val = self
                        .compile_expr(&args[1], function)?
                        .unwrap()
                        .into_struct_value();
                    let pp = self.builder.build_extract_value(path_val, 0, "wf_pp").unwrap();
                    let pl = self.builder.build_extract_value(path_val, 1, "wf_pl").unwrap();
                    let dp = self.builder.build_extract_value(data_val, 0, "wf_dp").unwrap();
                    let dl = self.builder.build_extract_value(data_val, 1, "wf_dl").unwrap();
                    let wf_fn = self.get_or_declare_ny_write_file();
                    let result = self
                        .builder
                        .build_call(
                            wf_fn,
                            &[pp.into(), pl.into(), dp.into(), dl.into()],
                            "wf_r",
                        )
                        .unwrap()
                        .try_as_basic_value()
                        .basic()
                        .unwrap();
                    return Ok(Some(result));
                }

                // float_to_str(f64) -> str
                if callee == "float_to_str" {
                    let val = self
                        .compile_expr(&args[0], function)?
                        .unwrap()
                        .into_float_value();
                    let f64_ty = self.context.f64_type();
                    let val_f64 = if val.get_type() != f64_ty {
                        self.builder.build_float_ext(val, f64_ty, "fts_ext").unwrap()
                    } else {
                        val
                    };
                    let i64_ty = self.context.i64_type();
                    let out_len_ptr = self.builder.build_alloca(i64_ty, "fts_len_p").unwrap();
                    let fts_fn = self.get_or_declare_ny_float_to_str();
                    let buf = self
                        .builder
                        .build_call(fts_fn, &[val_f64.into(), out_len_ptr.into()], "fts_buf")
                        .unwrap()
                        .try_as_basic_value()
                        .basic()
                        .unwrap()
                        .into_pointer_value();
                    let out_len = self
                        .builder
                        .build_load(i64_ty, out_len_ptr, "fts_len")
                        .unwrap()
                        .into_int_value();
                    let str_ty = str_type(self.context);
                    let result = str_ty.const_zero();
                    let result = self.builder.build_insert_value(result, buf, 0, "fts_p").unwrap();
                    let result = self.builder.build_insert_value(result, out_len, 1, "fts_l").unwrap();
                    return Ok(Some(result.into_struct_value().into()));
                }

                // str_to_float(str) -> f64
                if callee == "str_to_float" {
                    let str_val = self
                        .compile_expr(&args[0], function)?
                        .unwrap()
                        .into_struct_value();
                    let ptr = self.builder.build_extract_value(str_val, 0, "stf_p").unwrap();
                    let len = self.builder.build_extract_value(str_val, 1, "stf_l").unwrap();
                    let stf_fn = self.get_or_declare_ny_str_to_float();
                    let result = self
                        .builder
                        .build_call(stf_fn, &[ptr.into(), len.into()], "stf_r")
                        .unwrap()
                        .try_as_basic_value()
                        .basic()
                        .unwrap();
                    return Ok(Some(result));
                }

                // Math builtins (f64): sqrt, sin, cos, floor, ceil, fabs, log, exp, pow
                if matches!(
                    callee.as_str(),
                    "sqrt" | "sin" | "cos" | "floor" | "ceil" | "fabs" | "log" | "exp"
                ) {
                    let arg = self
                        .compile_expr(&args[0], function)?
                        .unwrap()
                        .into_float_value();
                    let f64_ty = self.context.f64_type();
                    let arg_f64 = if arg.get_type() != f64_ty {
                        self.builder
                            .build_float_ext(arg, f64_ty, "to_f64")
                            .unwrap()
                    } else {
                        arg
                    };
                    let fn_ty = f64_ty.fn_type(&[f64_ty.into()], false);
                    let math_fn = self
                        .module
                        .get_function(callee)
                        .unwrap_or_else(|| self.module.add_function(callee, fn_ty, None));
                    let result = self
                        .builder
                        .build_call(math_fn, &[arg_f64.into()], callee)
                        .unwrap()
                        .try_as_basic_value()
                        .basic()
                        .unwrap();
                    return Ok(Some(result));
                }

                if callee == "pow" {
                    let base = self
                        .compile_expr(&args[0], function)?
                        .unwrap()
                        .into_float_value();
                    let exp = self
                        .compile_expr(&args[1], function)?
                        .unwrap()
                        .into_float_value();
                    let f64_ty = self.context.f64_type();
                    let base_f64 = if base.get_type() != f64_ty {
                        self.builder.build_float_ext(base, f64_ty, "b64").unwrap()
                    } else {
                        base
                    };
                    let exp_f64 = if exp.get_type() != f64_ty {
                        self.builder.build_float_ext(exp, f64_ty, "e64").unwrap()
                    } else {
                        exp
                    };
                    let fn_ty = f64_ty.fn_type(&[f64_ty.into(), f64_ty.into()], false);
                    let pow_fn = self
                        .module
                        .get_function("pow")
                        .unwrap_or_else(|| self.module.add_function("pow", fn_ty, None));
                    let result = self
                        .builder
                        .build_call(pow_fn, &[base_f64.into(), exp_f64.into()], "pow")
                        .unwrap()
                        .try_as_basic_value()
                        .basic()
                        .unwrap();
                    return Ok(Some(result));
                }

                // Handle clock_ms() — monotonic millisecond timer
                if callee == "clock_ms" {
                    let clock_fn = self.get_or_declare_ny_clock_ms();
                    let result = self
                        .builder
                        .build_call(clock_fn, &[], "clock_ms")
                        .unwrap()
                        .try_as_basic_value()
                        .basic()
                        .unwrap();
                    return Ok(Some(result));
                }

                // Handle sizeof(Type) builtin — compile-time size
                if callee == "sizeof" {
                    // sizeof takes 1 arg — we infer its type and return the size
                    let arg_ty = self.infer_expr_type(&args[0]);
                    let llvm_ty = ny_to_llvm(self.context, &arg_ty);
                    let size = llvm_ty.size_of().unwrap();
                    return Ok(Some(size.into()));
                }

                if let Some((func, _, ret_ty)) = self.functions.get(callee).cloned() {
                    let mut arg_values = Vec::new();
                    for arg in args {
                        let val = self.compile_expr(arg, function)?.unwrap();
                        arg_values.push(val.into());
                    }
                    let call = self.builder.build_call(func, &arg_values, "call").unwrap();

                    if ret_ty == NyType::Unit {
                        Ok(None)
                    } else {
                        Ok(call.try_as_basic_value().basic())
                    }
                } else if let Some((ptr, var_ty)) = self.variables.get(callee).cloned() {
                    // Check if this is a capturing closure
                    if let Some((lambda_fn_name, cap_vars)) =
                        self.closure_captures.get(callee).cloned()
                    {
                        // Call the lambda function directly with captures prepended
                        if let Some((func, _, ret_ty)) =
                            self.functions.get(&lambda_fn_name).cloned()
                        {
                            let mut arg_values: Vec<inkwell::values::BasicMetadataValueEnum> =
                                Vec::new();
                            // Load and pass captured values
                            for (cap_name, cap_ty) in &cap_vars {
                                if let Some((cap_ptr, _)) = self.variables.get(cap_name) {
                                    let cap_llvm = ny_to_llvm(self.context, cap_ty);
                                    let cap_val = self
                                        .builder
                                        .build_load(cap_llvm, *cap_ptr, cap_name)
                                        .unwrap();
                                    arg_values.push(cap_val.into());
                                }
                            }
                            // Pass explicit args
                            for arg in args {
                                let val = self.compile_expr(arg, function)?.unwrap();
                                arg_values.push(val.into());
                            }
                            let call = self
                                .builder
                                .build_call(func, &arg_values, "closure_call")
                                .unwrap();
                            return if ret_ty == NyType::Unit {
                                Ok(None)
                            } else {
                                Ok(call.try_as_basic_value().basic())
                            };
                        }
                    }

                    // Calling a function pointer variable
                    if let NyType::Function {
                        params: param_tys,
                        ret,
                    } = &var_ty
                    {
                        let llvm_param_types: Vec<BasicTypeEnum> = param_tys
                            .iter()
                            .map(|t| ny_to_llvm(self.context, t))
                            .collect();
                        let param_meta: Vec<_> =
                            llvm_param_types.iter().map(|t| (*t).into()).collect();
                        let fn_type = match ret.as_ref() {
                            NyType::Unit => self.context.void_type().fn_type(&param_meta, false),
                            ty => ny_to_llvm(self.context, ty).fn_type(&param_meta, false),
                        };

                        // Load the function pointer
                        let fn_ptr_val = self
                            .builder
                            .build_load(
                                self.context.ptr_type(AddressSpace::default()),
                                ptr,
                                "fn_ptr",
                            )
                            .unwrap()
                            .into_pointer_value();

                        let mut arg_values = Vec::new();
                        for arg in args {
                            let val = self.compile_expr(arg, function)?.unwrap();
                            arg_values.push(val.into());
                        }

                        let call = self
                            .builder
                            .build_indirect_call(fn_type, fn_ptr_val, &arg_values, "lcall")
                            .unwrap();

                        if **ret == NyType::Unit {
                            Ok(None)
                        } else {
                            Ok(call.try_as_basic_value().basic())
                        }
                    } else {
                        Err(vec![CompileError::type_error(
                            format!("'{}' is not callable", callee),
                            expr.span(),
                        )])
                    }
                } else {
                    Err(vec![CompileError::name_error(
                        format!("undeclared function '{}'", callee),
                        expr.span(),
                    )])
                }
            }

            // ---- If/else ----
            Expr::If {
                condition,
                then_branch,
                else_branch,
                ..
            } => {
                let cond_val = self.compile_expr(condition, function)?.unwrap();
                let cond_int = cond_val.into_int_value();

                let then_bb = self.context.append_basic_block(*function, "then");
                let else_bb = self.context.append_basic_block(*function, "else");
                let merge_bb = self.context.append_basic_block(*function, "merge");

                self.builder
                    .build_conditional_branch(cond_int, then_bb, else_bb)
                    .unwrap();

                // Then branch
                self.builder.position_at_end(then_bb);
                let then_val = self.compile_expr(then_branch, function)?;
                let then_has_terminator = self
                    .builder
                    .get_insert_block()
                    .unwrap()
                    .get_terminator()
                    .is_some();
                let then_end_bb = self.builder.get_insert_block().unwrap();
                if !then_has_terminator {
                    self.builder.build_unconditional_branch(merge_bb).unwrap();
                }

                // Else branch
                self.builder.position_at_end(else_bb);
                let else_val = if let Some(eb) = else_branch {
                    self.compile_expr(eb, function)?
                } else {
                    None
                };
                let else_has_terminator = self
                    .builder
                    .get_insert_block()
                    .unwrap()
                    .get_terminator()
                    .is_some();
                let else_end_bb = self.builder.get_insert_block().unwrap();
                if !else_has_terminator {
                    self.builder.build_unconditional_branch(merge_bb).unwrap();
                }

                self.builder.position_at_end(merge_bb);

                // Build phi if both branches return values and don't have terminators
                if let (Some(tv), Some(ev)) = (&then_val, &else_val) {
                    if !then_has_terminator && !else_has_terminator {
                        let phi = self.builder.build_phi(tv.get_type(), "ifval").unwrap();
                        phi.add_incoming(&[(tv, then_end_bb), (ev, else_end_bb)]);
                        return Ok(Some(phi.as_basic_value()));
                    }
                }

                Ok(None)
            }

            // ---- Block ----
            Expr::Block {
                stmts, tail_expr, ..
            } => {
                for stmt in stmts {
                    self.compile_stmt(stmt, function)?;
                    // If the block has been terminated (return/break/continue), stop
                    if self
                        .builder
                        .get_insert_block()
                        .unwrap()
                        .get_terminator()
                        .is_some()
                    {
                        return Ok(None);
                    }
                }
                if let Some(expr) = tail_expr {
                    self.compile_expr(expr, function)
                } else {
                    Ok(None)
                }
            }

            // ---- Array literal ----
            Expr::ArrayLit { elements, .. } => {
                if elements.is_empty() {
                    // Zero-length array: just return an i32 array of size 0
                    let arr_ty = self.context.i32_type().array_type(0);
                    let alloca = self.builder.build_alloca(arr_ty, "arr").unwrap();
                    return Ok(Some(
                        self.builder.build_load(arr_ty, alloca, "arr_val").unwrap(),
                    ));
                }

                // Compile all elements
                let mut elem_values = Vec::new();
                for elem in elements {
                    let val = self.compile_expr(elem, function)?.unwrap();
                    elem_values.push(val);
                }

                let elem_ty = elem_values[0].get_type();
                let arr_size = elem_values.len() as u32;
                let arr_ty = elem_ty.array_type(arr_size);
                let alloca = self.builder.build_alloca(arr_ty, "arr").unwrap();

                let zero = self.context.i64_type().const_zero();
                for (i, val) in elem_values.iter().enumerate() {
                    let idx = self.context.i64_type().const_int(i as u64, false);
                    let gep = unsafe {
                        self.builder
                            .build_in_bounds_gep(arr_ty, alloca, &[zero, idx], "arr_elem_ptr")
                            .unwrap()
                    };
                    self.builder.build_store(gep, *val).unwrap();
                }

                // Load the whole array value
                let arr_val = self.builder.build_load(arr_ty, alloca, "arr_val").unwrap();
                Ok(Some(arr_val))
            }

            // ---- Index access (read) ----
            Expr::Index { object, index, .. } => {
                let obj_ty = self.infer_expr_type(object);
                let idx_val = self
                    .compile_expr(index, function)?
                    .unwrap()
                    .into_int_value();

                match &obj_ty {
                    NyType::Array { elem, size } => {
                        let elem_llvm = ny_to_llvm(self.context, elem);
                        let arr_llvm = elem_llvm.array_type(*size as u32);

                        // Get pointer to the array (we need the alloca, not the loaded value)
                        let arr_ptr = self.compile_expr_as_ptr(object, function)?;

                        // Bounds check
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
                                .build_in_bounds_gep(arr_llvm, arr_ptr, &[zero, idx_i64], "idx_ptr")
                                .unwrap()
                        };

                        let val = self.builder.build_load(elem_llvm, gep, "idx_val").unwrap();
                        Ok(Some(val))
                    }
                    NyType::Slice(elem) => {
                        // Slice indexing: extract ptr from {ptr, len}, GEP, load
                        let elem_llvm = ny_to_llvm(self.context, elem);
                        let slice_val = self
                            .compile_expr(object, function)?
                            .unwrap()
                            .into_struct_value();
                        let ptr = self
                            .builder
                            .build_extract_value(slice_val, 0, "slice_ptr")
                            .unwrap()
                            .into_pointer_value();
                        let len = self
                            .builder
                            .build_extract_value(slice_val, 1, "slice_len")
                            .unwrap()
                            .into_int_value();

                        let idx_i64 = self
                            .builder
                            .build_int_z_extend_or_bit_cast(
                                idx_val,
                                self.context.i64_type(),
                                "idx_ext",
                            )
                            .unwrap();
                        self.build_bounds_check(idx_i64, len, function);

                        let gep = unsafe {
                            self.builder
                                .build_in_bounds_gep(elem_llvm, ptr, &[idx_i64], "slice_idx_ptr")
                                .unwrap()
                        };
                        let val = self
                            .builder
                            .build_load(elem_llvm, gep, "slice_idx_val")
                            .unwrap();
                        Ok(Some(val))
                    }
                    _ => Err(vec![CompileError::type_error(
                        "cannot index non-array type".to_string(),
                        object.span(),
                    )]),
                }
            }

            // ---- Struct initialization ----
            Expr::StructInit { name, fields, .. } => {
                let struct_fields = self.struct_types.get(name).cloned().unwrap_or_default();
                let struct_ty = self.get_or_create_llvm_struct_type(name, &struct_fields);
                let alloca = self
                    .builder
                    .build_alloca(struct_ty, &format!("{}_init", name))
                    .unwrap();

                // Store each field value at the correct index
                for (field_name, field_expr) in fields {
                    let field_idx = struct_fields
                        .iter()
                        .position(|(n, _)| n == field_name)
                        .unwrap_or(0) as u32;
                    let val = self.compile_expr(field_expr, function)?.unwrap();
                    let field_ptr = self
                        .builder
                        .build_struct_gep(
                            struct_ty,
                            alloca,
                            field_idx,
                            &format!("{}_ptr", field_name),
                        )
                        .unwrap();
                    self.builder.build_store(field_ptr, val).unwrap();
                }

                // Load the whole struct value
                let struct_val = self
                    .builder
                    .build_load(struct_ty, alloca, &format!("{}_val", name))
                    .unwrap();
                Ok(Some(struct_val))
            }

            // ---- Field access ----
            Expr::FieldAccess { object, field, .. } => {
                let obj_ty = self.infer_expr_type(object);

                // Handle auto-deref for pointers to structs
                let (struct_name, struct_fields, is_pointer) = match &obj_ty {
                    NyType::Struct { name, fields } => (name.clone(), fields.clone(), false),
                    NyType::Pointer(inner) => match inner.as_ref() {
                        NyType::Struct { name, fields } => (name.clone(), fields.clone(), true),
                        _ => {
                            return Err(vec![CompileError::type_error(
                                "field access on non-struct pointer".to_string(),
                                object.span(),
                            )]);
                        }
                    },
                    _ => {
                        return Err(vec![CompileError::type_error(
                            format!("field access on non-struct type '{}'", obj_ty),
                            object.span(),
                        )]);
                    }
                };

                let field_idx = struct_fields
                    .iter()
                    .position(|(n, _)| n == field)
                    .unwrap_or(0) as u32;
                let field_ty_ny = struct_fields
                    .get(field_idx as usize)
                    .map(|(_, t)| t.clone())
                    .unwrap_or(NyType::I32);
                let field_ty_llvm = ny_to_llvm(self.context, &field_ty_ny);

                let struct_llvm_ty =
                    self.get_or_create_llvm_struct_type(&struct_name, &struct_fields);

                let struct_ptr = if is_pointer {
                    // Auto-deref: the object is a pointer to the struct, load the pointer
                    let ptr_val = self.compile_expr(object, function)?.unwrap();
                    ptr_val.into_pointer_value()
                } else {
                    // The object is a struct value; get its alloca pointer
                    self.compile_expr_as_ptr(object, function)?
                };

                let field_ptr = self
                    .builder
                    .build_struct_gep(
                        struct_llvm_ty,
                        struct_ptr,
                        field_idx,
                        &format!("{}_field", field),
                    )
                    .unwrap();

                let val = self
                    .builder
                    .build_load(field_ty_llvm, field_ptr, field)
                    .unwrap();
                Ok(Some(val))
            }

            // ---- Address-of (&expr) ----
            Expr::AddrOf { operand, .. } => {
                let ptr = self.compile_expr_as_ptr(operand, function)?;
                Ok(Some(ptr.into()))
            }

            // ---- Dereference (*expr) ----
            Expr::Deref { operand, .. } => {
                let ptr_val = self.compile_expr(operand, function)?.unwrap();
                let ptr = ptr_val.into_pointer_value();
                let pointee_ty = self.infer_expr_type(expr);
                let llvm_ty = ny_to_llvm(self.context, &pointee_ty);
                let val = self.builder.build_load(llvm_ty, ptr, "deref").unwrap();
                Ok(Some(val))
            }

            // ---- Method call ----
            Expr::MethodCall {
                object,
                method,
                args,
                ..
            } => {
                let obj_ty = self.infer_expr_type(object);

                // Handle HashMap<K,V> methods
                if let NyType::HashMap(_key_ty, val_ty) = &obj_ty {
                    let val_llvm = ny_to_llvm(self.context, val_ty);
                    // Buffer type for C runtime: always 16 bytes (max value size)
                    let buf_ty = self.context.i8_type().array_type(16);

                    match method.as_str() {
                        "insert" => {
                            let map_ptr = self.compile_expr(object, function)?.unwrap();
                            let key_val = self.compile_expr(&args[0], function)?.unwrap().into_struct_value();
                            let kp = self.builder.build_extract_value(key_val, 0, "hi_kp").unwrap();
                            let kl = self.builder.build_extract_value(key_val, 1, "hi_kl").unwrap();
                            let val = self.compile_expr(&args[1], function)?.unwrap();
                            // Alloca 16-byte buffer, store value at start
                            let val_alloca = self.builder.build_alloca(buf_ty, "hi_buf").unwrap();
                            self.builder.build_store(val_alloca, val).unwrap();
                            let fn_val = self.get_or_declare_ny_hmap_insert();
                            self.builder.build_call(fn_val, &[map_ptr.into(), kp.into(), kl.into(), val_alloca.into()], "").unwrap();
                            return Ok(None);
                        }
                        "get" => {
                            let map_ptr = self.compile_expr(object, function)?.unwrap();
                            let key_val = self.compile_expr(&args[0], function)?.unwrap().into_struct_value();
                            let kp = self.builder.build_extract_value(key_val, 0, "hg_kp").unwrap();
                            let kl = self.builder.build_extract_value(key_val, 1, "hg_kl").unwrap();
                            // Alloca 16-byte buffer, read value
                            let out_alloca = self.builder.build_alloca(buf_ty, "hg_buf").unwrap();
                            let fn_val = self.get_or_declare_ny_hmap_get();
                            self.builder.build_call(fn_val, &[map_ptr.into(), kp.into(), kl.into(), out_alloca.into()], "").unwrap();
                            let result = self.builder.build_load(val_llvm, out_alloca, "hg_val").unwrap();
                            return Ok(Some(result));
                        }
                        "contains" => {
                            let map_ptr = self.compile_expr(object, function)?.unwrap();
                            let key_val = self.compile_expr(&args[0], function)?.unwrap().into_struct_value();
                            let kp = self.builder.build_extract_value(key_val, 0, "hc_kp").unwrap();
                            let kl = self.builder.build_extract_value(key_val, 1, "hc_kl").unwrap();
                            let fn_val = self.get_or_declare_ny_hmap_contains();
                            let result = self.builder.build_call(fn_val, &[map_ptr.into(), kp.into(), kl.into()], "hc_r").unwrap()
                                .try_as_basic_value().basic().unwrap();
                            return Ok(Some(result));
                        }
                        "len" => {
                            let map_ptr = self.compile_expr(object, function)?.unwrap();
                            let fn_val = self.get_or_declare_ny_hmap_len();
                            let result = self.builder.build_call(fn_val, &[map_ptr.into()], "hl_r").unwrap()
                                .try_as_basic_value().basic().unwrap();
                            return Ok(Some(result));
                        }
                        "remove" => {
                            let map_ptr = self.compile_expr(object, function)?.unwrap();
                            let key_val = self.compile_expr(&args[0], function)?.unwrap().into_struct_value();
                            let kp = self.builder.build_extract_value(key_val, 0, "hr_kp").unwrap();
                            let kl = self.builder.build_extract_value(key_val, 1, "hr_kl").unwrap();
                            let fn_val = self.get_or_declare_ny_hmap_remove();
                            self.builder.build_call(fn_val, &[map_ptr.into(), kp.into(), kl.into()], "").unwrap();
                            return Ok(None);
                        }
                        "free" => {
                            let map_ptr = self.compile_expr(object, function)?.unwrap();
                            let fn_val = self.get_or_declare_ny_hmap_free();
                            self.builder.build_call(fn_val, &[map_ptr.into()], "").unwrap();
                            return Ok(None);
                        }
                        _ => {}
                    }
                }

                // Handle built-in Vec methods
                if let NyType::Vec(elem_ty) = &obj_ty {
                    let elem_llvm = ny_to_llvm(self.context, elem_ty);
                    let vec_struct_ty = ny_to_llvm(self.context, &obj_ty).into_struct_type();

                    match method.as_str() {
                        "len" => {
                            let obj_val = self.compile_expr(object, function)?.unwrap();
                            let sv = obj_val.into_struct_value();
                            let len = self.builder.build_extract_value(sv, 1, "vec_len").unwrap();
                            return Ok(Some(len));
                        }
                        "get" => {
                            let obj_val = self.compile_expr(object, function)?.unwrap();
                            let sv = obj_val.into_struct_value();
                            let data_ptr = self
                                .builder
                                .build_extract_value(sv, 0, "vec_data")
                                .unwrap()
                                .into_pointer_value();
                            let len = self
                                .builder
                                .build_extract_value(sv, 1, "vec_len_check")
                                .unwrap()
                                .into_int_value();
                            let idx = self
                                .compile_expr(&args[0], function)?
                                .unwrap()
                                .into_int_value();
                            let idx_i64 = self
                                .builder
                                .build_int_z_extend_or_bit_cast(
                                    idx,
                                    self.context.i64_type(),
                                    "idx64",
                                )
                                .unwrap();
                            // Bounds check
                            self.build_bounds_check(idx_i64, len, function);
                            let gep = unsafe {
                                self.builder
                                    .build_in_bounds_gep(
                                        elem_llvm,
                                        data_ptr,
                                        &[idx_i64],
                                        "vec_elem_ptr",
                                    )
                                    .unwrap()
                            };
                            let val = self.builder.build_load(elem_llvm, gep, "vec_elem").unwrap();
                            return Ok(Some(val));
                        }
                        "set" => {
                            // v.set(index, value) — write to existing element
                            let vec_ptr = self.compile_expr_as_ptr(object, function)?;
                            let idx = self
                                .compile_expr(&args[0], function)?
                                .unwrap()
                                .into_int_value();
                            let val = self.compile_expr(&args[1], function)?.unwrap();
                            let idx_i64 = self
                                .builder
                                .build_int_z_extend_or_bit_cast(
                                    idx,
                                    self.context.i64_type(),
                                    "set_idx64",
                                )
                                .unwrap();

                            // Load data ptr and len for bounds check
                            let data_gep = self
                                .builder
                                .build_struct_gep(vec_struct_ty, vec_ptr, 0, "set_data_gep")
                                .unwrap();
                            let len_gep = self
                                .builder
                                .build_struct_gep(vec_struct_ty, vec_ptr, 1, "set_len_gep")
                                .unwrap();
                            let data_ptr = self
                                .builder
                                .build_load(
                                    self.context.ptr_type(AddressSpace::default()),
                                    data_gep,
                                    "set_data",
                                )
                                .unwrap()
                                .into_pointer_value();
                            let len = self
                                .builder
                                .build_load(self.context.i64_type(), len_gep, "set_len")
                                .unwrap()
                                .into_int_value();

                            // Bounds check
                            self.build_bounds_check(idx_i64, len, function);

                            let elem_ptr = unsafe {
                                self.builder
                                    .build_in_bounds_gep(
                                        elem_llvm,
                                        data_ptr,
                                        &[idx_i64],
                                        "set_elem_ptr",
                                    )
                                    .unwrap()
                            };
                            self.builder.build_store(elem_ptr, val).unwrap();

                            return Ok(None);
                        }
                        "push" => {
                            // Need the alloca pointer to mutate the vec
                            let vec_ptr = self.compile_expr_as_ptr(object, function)?;
                            let val = self.compile_expr(&args[0], function)?.unwrap();

                            // Load current data, len, cap
                            let data_gep = self
                                .builder
                                .build_struct_gep(vec_struct_ty, vec_ptr, 0, "data_gep")
                                .unwrap();
                            let len_gep = self
                                .builder
                                .build_struct_gep(vec_struct_ty, vec_ptr, 1, "len_gep")
                                .unwrap();
                            let cap_gep = self
                                .builder
                                .build_struct_gep(vec_struct_ty, vec_ptr, 2, "cap_gep")
                                .unwrap();

                            let data_ptr = self
                                .builder
                                .build_load(
                                    self.context.ptr_type(AddressSpace::default()),
                                    data_gep,
                                    "data",
                                )
                                .unwrap()
                                .into_pointer_value();
                            let len = self
                                .builder
                                .build_load(self.context.i64_type(), len_gep, "len")
                                .unwrap()
                                .into_int_value();
                            let cap = self
                                .builder
                                .build_load(self.context.i64_type(), cap_gep, "cap")
                                .unwrap()
                                .into_int_value();

                            // Check if we need to grow: if len >= cap, realloc
                            let needs_grow = self
                                .builder
                                .build_int_compare(IntPredicate::UGE, len, cap, "needs_grow")
                                .unwrap();

                            let grow_bb = self.context.append_basic_block(*function, "vec_grow");
                            let push_bb = self.context.append_basic_block(*function, "vec_push");

                            self.builder
                                .build_conditional_branch(needs_grow, grow_bb, push_bb)
                                .unwrap();

                            // Grow: double capacity, realloc
                            self.builder.position_at_end(grow_bb);
                            let new_cap = self
                                .builder
                                .build_int_mul(
                                    cap,
                                    self.context.i64_type().const_int(2, false),
                                    "new_cap",
                                )
                                .unwrap();
                            let elem_size = elem_llvm.size_of().unwrap();
                            let new_size = self
                                .builder
                                .build_int_mul(new_cap, elem_size, "new_size")
                                .unwrap();
                            let realloc_fn = self.get_or_declare_realloc();
                            let new_data = self
                                .builder
                                .build_call(
                                    realloc_fn,
                                    &[data_ptr.into(), new_size.into()],
                                    "new_data",
                                )
                                .unwrap()
                                .try_as_basic_value()
                                .basic()
                                .unwrap()
                                .into_pointer_value();
                            self.builder.build_store(data_gep, new_data).unwrap();
                            self.builder.build_store(cap_gep, new_cap).unwrap();
                            self.builder.build_unconditional_branch(push_bb).unwrap();

                            // Push: store value at data[len], increment len
                            self.builder.position_at_end(push_bb);
                            // Re-load data ptr (may have changed from realloc)
                            let current_data = self
                                .builder
                                .build_load(
                                    self.context.ptr_type(AddressSpace::default()),
                                    data_gep,
                                    "cur_data",
                                )
                                .unwrap()
                                .into_pointer_value();
                            let current_len = self
                                .builder
                                .build_load(self.context.i64_type(), len_gep, "cur_len")
                                .unwrap()
                                .into_int_value();

                            let elem_ptr = unsafe {
                                self.builder
                                    .build_in_bounds_gep(
                                        elem_llvm,
                                        current_data,
                                        &[current_len],
                                        "push_ptr",
                                    )
                                    .unwrap()
                            };
                            self.builder.build_store(elem_ptr, val).unwrap();

                            // len += 1
                            let new_len = self
                                .builder
                                .build_int_add(
                                    current_len,
                                    self.context.i64_type().const_int(1, false),
                                    "new_len",
                                )
                                .unwrap();
                            self.builder.build_store(len_gep, new_len).unwrap();

                            return Ok(None);
                        }
                        "pop" => {
                            // v.pop() — remove and return last element
                            let vec_ptr = self.compile_expr_as_ptr(object, function)?;

                            let len_gep = self
                                .builder
                                .build_struct_gep(vec_struct_ty, vec_ptr, 1, "pop_len_gep")
                                .unwrap();
                            let data_gep = self
                                .builder
                                .build_struct_gep(vec_struct_ty, vec_ptr, 0, "pop_data_gep")
                                .unwrap();

                            let len = self
                                .builder
                                .build_load(self.context.i64_type(), len_gep, "pop_len")
                                .unwrap()
                                .into_int_value();
                            let data_ptr = self
                                .builder
                                .build_load(
                                    self.context.ptr_type(AddressSpace::default()),
                                    data_gep,
                                    "pop_data",
                                )
                                .unwrap()
                                .into_pointer_value();

                            // Bounds check: len > 0 (use len-1 as index, check < len)
                            let one = self.context.i64_type().const_int(1, false);
                            let last_idx = self
                                .builder
                                .build_int_sub(len, one, "last_idx")
                                .unwrap();
                            self.build_bounds_check(last_idx, len, function);

                            // Load data[len-1]
                            let elem_ptr = unsafe {
                                self.builder
                                    .build_in_bounds_gep(
                                        elem_llvm,
                                        data_ptr,
                                        &[last_idx],
                                        "pop_elem_ptr",
                                    )
                                    .unwrap()
                            };
                            let val = self
                                .builder
                                .build_load(elem_llvm, elem_ptr, "pop_val")
                                .unwrap();

                            // Decrement len
                            let new_len = self
                                .builder
                                .build_int_sub(len, one, "pop_new_len")
                                .unwrap();
                            self.builder.build_store(len_gep, new_len).unwrap();

                            return Ok(Some(val));
                        }
                        "clear" => {
                            // v.clear() — reset length to 0
                            let vec_ptr = self.compile_expr_as_ptr(object, function)?;
                            let len_gep = self
                                .builder
                                .build_struct_gep(vec_struct_ty, vec_ptr, 1, "clear_len_gep")
                                .unwrap();
                            let zero = self.context.i64_type().const_int(0, false);
                            self.builder.build_store(len_gep, zero).unwrap();
                            return Ok(None);
                        }
                        "reverse" => {
                            // v.reverse() — in-place reverse
                            let vec_ptr = self.compile_expr_as_ptr(object, function)?;
                            let len_gep = self
                                .builder
                                .build_struct_gep(vec_struct_ty, vec_ptr, 1, "rev_len_gep")
                                .unwrap();
                            let data_gep = self
                                .builder
                                .build_struct_gep(vec_struct_ty, vec_ptr, 0, "rev_data_gep")
                                .unwrap();
                            let len = self
                                .builder
                                .build_load(self.context.i64_type(), len_gep, "rev_len")
                                .unwrap()
                                .into_int_value();
                            let data_ptr = self
                                .builder
                                .build_load(
                                    self.context.ptr_type(AddressSpace::default()),
                                    data_gep,
                                    "rev_data",
                                )
                                .unwrap()
                                .into_pointer_value();

                            let i64_ty = self.context.i64_type();
                            let zero = i64_ty.const_int(0, false);
                            let one = i64_ty.const_int(1, false);
                            let two = i64_ty.const_int(2, false);

                            let half = self
                                .builder
                                .build_int_unsigned_div(len, two, "half")
                                .unwrap();

                            let pre_bb = self.builder.get_insert_block().unwrap();
                            let loop_bb =
                                self.context.append_basic_block(*function, "rev_loop");
                            let swap_bb =
                                self.context.append_basic_block(*function, "rev_swap");
                            let done_bb =
                                self.context.append_basic_block(*function, "rev_done");

                            self.builder.build_unconditional_branch(loop_bb).unwrap();

                            self.builder.position_at_end(loop_bb);
                            let i_phi =
                                self.builder.build_phi(i64_ty, "rev_i").unwrap();
                            i_phi.add_incoming(&[(&zero, pre_bb)]);
                            let i_val = i_phi.as_basic_value().into_int_value();
                            let cond = self
                                .builder
                                .build_int_compare(IntPredicate::ULT, i_val, half, "rev_cond")
                                .unwrap();
                            self.builder
                                .build_conditional_branch(cond, swap_bb, done_bb)
                                .unwrap();

                            self.builder.position_at_end(swap_bb);
                            let j_val = self
                                .builder
                                .build_int_sub(
                                    self.builder.build_int_sub(len, one, "lm1").unwrap(),
                                    i_val,
                                    "rev_j",
                                )
                                .unwrap();
                            let ptr_i = unsafe {
                                self.builder
                                    .build_in_bounds_gep(
                                        elem_llvm, data_ptr, &[i_val], "rev_ptr_i",
                                    )
                                    .unwrap()
                            };
                            let ptr_j = unsafe {
                                self.builder
                                    .build_in_bounds_gep(
                                        elem_llvm, data_ptr, &[j_val], "rev_ptr_j",
                                    )
                                    .unwrap()
                            };
                            let val_i = self
                                .builder
                                .build_load(elem_llvm, ptr_i, "rev_vi")
                                .unwrap();
                            let val_j = self
                                .builder
                                .build_load(elem_llvm, ptr_j, "rev_vj")
                                .unwrap();
                            self.builder.build_store(ptr_i, val_j).unwrap();
                            self.builder.build_store(ptr_j, val_i).unwrap();
                            let next_i = self
                                .builder
                                .build_int_add(i_val, one, "rev_next")
                                .unwrap();
                            i_phi.add_incoming(&[(&next_i, swap_bb)]);
                            self.builder
                                .build_unconditional_branch(loop_bb)
                                .unwrap();

                            self.builder.position_at_end(done_bb);
                            return Ok(None);
                        }
                        "sort" => {
                            // v.sort() — in-place ascending bubble sort
                            let vec_ptr = self.compile_expr_as_ptr(object, function)?;
                            let len_gep = self
                                .builder
                                .build_struct_gep(vec_struct_ty, vec_ptr, 1, "sort_len_gep")
                                .unwrap();
                            let data_gep = self
                                .builder
                                .build_struct_gep(vec_struct_ty, vec_ptr, 0, "sort_data_gep")
                                .unwrap();
                            let len = self
                                .builder
                                .build_load(self.context.i64_type(), len_gep, "sort_len")
                                .unwrap()
                                .into_int_value();
                            let data_ptr = self
                                .builder
                                .build_load(
                                    self.context.ptr_type(AddressSpace::default()),
                                    data_gep,
                                    "sort_data",
                                )
                                .unwrap()
                                .into_pointer_value();

                            let i64_ty = self.context.i64_type();
                            let zero = i64_ty.const_int(0, false);
                            let one = i64_ty.const_int(1, false);

                            // Outer loop: for i in 0..len-1
                            let outer_bb =
                                self.context.append_basic_block(*function, "sort_outer");
                            let inner_bb =
                                self.context.append_basic_block(*function, "sort_inner");
                            let cmp_bb =
                                self.context.append_basic_block(*function, "sort_cmp");
                            let swap_bb =
                                self.context.append_basic_block(*function, "sort_swap");
                            let inner_inc_bb =
                                self.context.append_basic_block(*function, "sort_inner_inc");
                            let outer_inc_bb =
                                self.context.append_basic_block(*function, "sort_outer_inc");
                            let done_bb =
                                self.context.append_basic_block(*function, "sort_done");

                            let len_m1 =
                                self.builder.build_int_sub(len, one, "len_m1").unwrap();
                            let pre_bb = self.builder.get_insert_block().unwrap();
                            self.builder.build_unconditional_branch(outer_bb).unwrap();

                            // Outer loop header
                            self.builder.position_at_end(outer_bb);
                            let i_phi =
                                self.builder.build_phi(i64_ty, "sort_i").unwrap();
                            i_phi.add_incoming(&[(&zero, pre_bb)]);
                            let i_val = i_phi.as_basic_value().into_int_value();
                            let outer_cond = self
                                .builder
                                .build_int_compare(IntPredicate::ULT, i_val, len_m1, "i_lt")
                                .unwrap();
                            self.builder
                                .build_conditional_branch(outer_cond, inner_bb, done_bb)
                                .unwrap();

                            // Inner loop header: for j in 0..len-1-i
                            self.builder.position_at_end(inner_bb);
                            let j_phi =
                                self.builder.build_phi(i64_ty, "sort_j").unwrap();
                            j_phi.add_incoming(&[(&zero, outer_bb)]);
                            let j_val = j_phi.as_basic_value().into_int_value();
                            let inner_limit = self
                                .builder
                                .build_int_sub(len_m1, i_val, "inner_lim")
                                .unwrap();
                            let inner_cond = self
                                .builder
                                .build_int_compare(IntPredicate::ULT, j_val, inner_limit, "j_lt")
                                .unwrap();
                            self.builder
                                .build_conditional_branch(inner_cond, cmp_bb, outer_inc_bb)
                                .unwrap();

                            // Compare: if data[j] > data[j+1], swap
                            self.builder.position_at_end(cmp_bb);
                            let j_plus1 = self
                                .builder
                                .build_int_add(j_val, one, "j_p1")
                                .unwrap();
                            let ptr_j = unsafe {
                                self.builder
                                    .build_in_bounds_gep(
                                        elem_llvm, data_ptr, &[j_val], "ptr_j",
                                    )
                                    .unwrap()
                            };
                            let ptr_j1 = unsafe {
                                self.builder
                                    .build_in_bounds_gep(
                                        elem_llvm, data_ptr, &[j_plus1], "ptr_j1",
                                    )
                                    .unwrap()
                            };
                            let val_j = self
                                .builder
                                .build_load(elem_llvm, ptr_j, "val_j")
                                .unwrap();
                            let val_j1 = self
                                .builder
                                .build_load(elem_llvm, ptr_j1, "val_j1")
                                .unwrap();

                            // Compare based on element type
                            let should_swap = if elem_ty.is_float() {
                                self.builder
                                    .build_float_compare(
                                        inkwell::FloatPredicate::OGT,
                                        val_j.into_float_value(),
                                        val_j1.into_float_value(),
                                        "fgt",
                                    )
                                    .unwrap()
                            } else {
                                self.builder
                                    .build_int_compare(
                                        IntPredicate::SGT,
                                        val_j.into_int_value(),
                                        val_j1.into_int_value(),
                                        "igt",
                                    )
                                    .unwrap()
                            };
                            self.builder
                                .build_conditional_branch(should_swap, swap_bb, inner_inc_bb)
                                .unwrap();

                            // Swap: data[j] = data[j+1], data[j+1] = tmp
                            self.builder.position_at_end(swap_bb);
                            self.builder.build_store(ptr_j, val_j1).unwrap();
                            self.builder.build_store(ptr_j1, val_j).unwrap();
                            self.builder
                                .build_unconditional_branch(inner_inc_bb)
                                .unwrap();

                            // Inner increment
                            self.builder.position_at_end(inner_inc_bb);
                            let next_j = self
                                .builder
                                .build_int_add(j_val, one, "next_j")
                                .unwrap();
                            j_phi.add_incoming(&[(&next_j, inner_inc_bb)]);
                            self.builder
                                .build_unconditional_branch(inner_bb)
                                .unwrap();

                            // Outer increment
                            self.builder.position_at_end(outer_inc_bb);
                            let next_i = self
                                .builder
                                .build_int_add(i_val, one, "next_i")
                                .unwrap();
                            i_phi.add_incoming(&[(&next_i, outer_inc_bb)]);
                            self.builder
                                .build_unconditional_branch(outer_bb)
                                .unwrap();

                            // Done
                            self.builder.position_at_end(done_bb);
                            return Ok(None);
                        }
                        "map" => {
                            // v.map(fn) -> Vec<T> — apply function to each element
                            // Supports both named functions and capturing closures
                            let obj_val = self.compile_expr(object, function)?.unwrap();
                            let sv = obj_val.into_struct_value();
                            let data_ptr = self
                                .builder
                                .build_extract_value(sv, 0, "map_data")
                                .unwrap()
                                .into_pointer_value();
                            let len = self
                                .builder
                                .build_extract_value(sv, 1, "map_len")
                                .unwrap()
                                .into_int_value();

                            // Compile the function argument — may be a closure
                            let fn_ptr_val = self
                                .compile_expr(&args[0], function)?
                                .unwrap()
                                .into_pointer_value();

                            // Check if argument is a capturing closure
                            // Named closure: check closure_captures by ident
                            // Inline lambda: check if it was just registered (last entry)
                            let closure_info = if let Expr::Ident { name, .. } = &args[0] {
                                self.closure_captures.get(name).cloned()
                            } else if let Expr::Lambda { .. } = &args[0] {
                                // Find the most recently added closure capture
                                self.closure_captures
                                    .iter()
                                    .filter(|(k, _)| k.starts_with("__lambda_"))
                                    .max_by_key(|(k, _)| k.to_string())
                                    .map(|(_, v)| v.clone())
                            } else {
                                None
                            };

                            // Create result Vec inline
                            let i64_ty = self.context.i64_type();
                            let zero = i64_ty.const_int(0, false);
                            let one = i64_ty.const_int(1, false);
                            let initial_cap = i64_ty.const_int(8, false);
                            let elem_sz = elem_llvm.size_of().unwrap();
                            let alloc_sz = self
                                .builder
                                .build_int_mul(initial_cap, elem_sz, "map_alloc")
                                .unwrap();
                            let malloc_fn = self.get_or_declare_malloc();
                            let new_data = self
                                .builder
                                .build_call(malloc_fn, &[alloc_sz.into()], "map_data")
                                .unwrap()
                                .try_as_basic_value()
                                .basic()
                                .unwrap()
                                .into_pointer_value();

                            let new_vec_ptr = self
                                .builder
                                .build_alloca(vec_struct_ty, "map_vec")
                                .unwrap();
                            // Init: {data, len=0, cap=8, elem_size}
                            let dg = self.builder.build_struct_gep(vec_struct_ty, new_vec_ptr, 0, "mv_dg").unwrap();
                            self.builder.build_store(dg, new_data).unwrap();
                            let lg = self.builder.build_struct_gep(vec_struct_ty, new_vec_ptr, 1, "mv_lg").unwrap();
                            self.builder.build_store(lg, zero).unwrap();
                            let cg = self.builder.build_struct_gep(vec_struct_ty, new_vec_ptr, 2, "mv_cg").unwrap();
                            self.builder.build_store(cg, initial_cap).unwrap();
                            let eg = self.builder.build_struct_gep(vec_struct_ty, new_vec_ptr, 3, "mv_eg").unwrap();
                            self.builder.build_store(eg, elem_sz).unwrap();

                            let pre_bb = self.builder.get_insert_block().unwrap();
                            let loop_bb =
                                self.context.append_basic_block(*function, "map_loop");
                            let body_bb =
                                self.context.append_basic_block(*function, "map_body");
                            let done_bb =
                                self.context.append_basic_block(*function, "map_done");

                            self.builder.build_unconditional_branch(loop_bb).unwrap();

                            self.builder.position_at_end(loop_bb);
                            let i_phi =
                                self.builder.build_phi(i64_ty, "map_i").unwrap();
                            i_phi.add_incoming(&[(&zero, pre_bb)]);
                            let i_val = i_phi.as_basic_value().into_int_value();
                            let cond = self
                                .builder
                                .build_int_compare(IntPredicate::ULT, i_val, len, "map_cond")
                                .unwrap();
                            self.builder
                                .build_conditional_branch(cond, body_bb, done_bb)
                                .unwrap();

                            self.builder.position_at_end(body_bb);
                            let elem_ptr = unsafe {
                                self.builder
                                    .build_in_bounds_gep(
                                        elem_llvm, data_ptr, &[i_val], "map_ep",
                                    )
                                    .unwrap()
                            };
                            let elem_val = self
                                .builder
                                .build_load(elem_llvm, elem_ptr, "map_ev")
                                .unwrap();

                            // Call function — direct for closures, indirect for function ptrs
                            let mapped = if let Some((lambda_fn_name, cap_vars)) = &closure_info {
                                if let Some((func, _, _)) = self.functions.get(lambda_fn_name).cloned() {
                                    let mut call_args: Vec<inkwell::values::BasicMetadataValueEnum> = Vec::new();
                                    for (cap_name, cap_ty) in cap_vars {
                                        if let Some((cap_ptr, _)) = self.variables.get(cap_name) {
                                            let cap_llvm = ny_to_llvm(self.context, cap_ty);
                                            let cv = self.builder.build_load(cap_llvm, *cap_ptr, cap_name).unwrap();
                                            call_args.push(cv.into());
                                        }
                                    }
                                    call_args.push(elem_val.into());
                                    self.builder.build_call(func, &call_args, "map_r").unwrap()
                                        .try_as_basic_value().basic().unwrap()
                                } else {
                                    let fn_ty = elem_llvm.fn_type(&[elem_llvm.into()], false);
                                    self.builder.build_indirect_call(fn_ty, fn_ptr_val, &[elem_val.into()], "map_r").unwrap()
                                        .try_as_basic_value().basic().unwrap()
                                }
                            } else {
                                let fn_ty = elem_llvm.fn_type(&[elem_llvm.into()], false);
                                self.builder.build_indirect_call(fn_ty, fn_ptr_val, &[elem_val.into()], "map_r").unwrap()
                                    .try_as_basic_value().basic().unwrap()
                            };

                            // Push to new vec (inline push: load len, store at data[len], inc len)
                            let nv_data_gep = self
                                .builder
                                .build_struct_gep(vec_struct_ty, new_vec_ptr, 0, "nv_dg")
                                .unwrap();
                            let nv_len_gep = self
                                .builder
                                .build_struct_gep(vec_struct_ty, new_vec_ptr, 1, "nv_lg")
                                .unwrap();
                            let nv_cap_gep = self
                                .builder
                                .build_struct_gep(vec_struct_ty, new_vec_ptr, 2, "nv_cg")
                                .unwrap();

                            let nv_data = self
                                .builder
                                .build_load(
                                    self.context.ptr_type(AddressSpace::default()),
                                    nv_data_gep,
                                    "nv_d",
                                )
                                .unwrap()
                                .into_pointer_value();
                            let nv_len = self
                                .builder
                                .build_load(i64_ty, nv_len_gep, "nv_l")
                                .unwrap()
                                .into_int_value();
                            let nv_cap = self
                                .builder
                                .build_load(i64_ty, nv_cap_gep, "nv_c")
                                .unwrap()
                                .into_int_value();

                            // Check capacity, grow if needed
                            let needs_grow = self
                                .builder
                                .build_int_compare(IntPredicate::UGE, nv_len, nv_cap, "nv_grow")
                                .unwrap();
                            let grow_bb =
                                self.context.append_basic_block(*function, "map_grow");
                            let push_bb =
                                self.context.append_basic_block(*function, "map_push");
                            self.builder
                                .build_conditional_branch(needs_grow, grow_bb, push_bb)
                                .unwrap();

                            self.builder.position_at_end(grow_bb);
                            let new_cap = self
                                .builder
                                .build_int_mul(
                                    nv_cap,
                                    i64_ty.const_int(2, false),
                                    "nv_nc",
                                )
                                .unwrap();
                            let new_size = self
                                .builder
                                .build_int_mul(new_cap, elem_llvm.size_of().unwrap(), "nv_ns")
                                .unwrap();
                            let realloc_fn = self.get_or_declare_realloc();
                            let new_data = self
                                .builder
                                .build_call(
                                    realloc_fn,
                                    &[nv_data.into(), new_size.into()],
                                    "nv_nd",
                                )
                                .unwrap()
                                .try_as_basic_value()
                                .basic()
                                .unwrap()
                                .into_pointer_value();
                            self.builder.build_store(nv_data_gep, new_data).unwrap();
                            self.builder.build_store(nv_cap_gep, new_cap).unwrap();
                            self.builder
                                .build_unconditional_branch(push_bb)
                                .unwrap();

                            self.builder.position_at_end(push_bb);
                            let cur_data = self
                                .builder
                                .build_load(
                                    self.context.ptr_type(AddressSpace::default()),
                                    nv_data_gep,
                                    "nv_cd",
                                )
                                .unwrap()
                                .into_pointer_value();
                            let cur_len = self
                                .builder
                                .build_load(i64_ty, nv_len_gep, "nv_cl")
                                .unwrap()
                                .into_int_value();
                            let dest = unsafe {
                                self.builder
                                    .build_in_bounds_gep(
                                        elem_llvm, cur_data, &[cur_len], "nv_dp",
                                    )
                                    .unwrap()
                            };
                            self.builder.build_store(dest, mapped).unwrap();
                            let new_len = self
                                .builder
                                .build_int_add(cur_len, one, "nv_nl")
                                .unwrap();
                            self.builder.build_store(nv_len_gep, new_len).unwrap();

                            let next_i = self
                                .builder
                                .build_int_add(i_val, one, "map_next")
                                .unwrap();
                            i_phi.add_incoming(&[(&next_i, push_bb)]);
                            self.builder
                                .build_unconditional_branch(loop_bb)
                                .unwrap();

                            self.builder.position_at_end(done_bb);
                            let result =
                                self.builder
                                    .build_load(vec_struct_ty, new_vec_ptr, "map_result")
                                    .unwrap();
                            return Ok(Some(result));
                        }
                        "filter" => {
                            // v.filter(predicate_fn) -> Vec<T> — keep elements where fn returns true
                            let obj_val = self.compile_expr(object, function)?.unwrap();
                            let sv = obj_val.into_struct_value();
                            let data_ptr = self
                                .builder
                                .build_extract_value(sv, 0, "fil_data")
                                .unwrap()
                                .into_pointer_value();
                            let len = self
                                .builder
                                .build_extract_value(sv, 1, "fil_len")
                                .unwrap()
                                .into_int_value();
                            let fn_ptr_val = self
                                .compile_expr(&args[0], function)?
                                .unwrap()
                                .into_pointer_value();
                            let closure_info = if let Expr::Ident { name, .. } = &args[0] {
                                self.closure_captures.get(name).cloned()
                            } else if let Expr::Lambda { .. } = &args[0] {
                                self.closure_captures
                                    .iter()
                                    .filter(|(k, _)| k.starts_with("__lambda_"))
                                    .max_by_key(|(k, _)| k.to_string())
                                    .map(|(_, v)| v.clone())
                            } else {
                                None
                            };

                            let i64_ty = self.context.i64_type();
                            let zero = i64_ty.const_int(0, false);
                            let one = i64_ty.const_int(1, false);
                            let initial_cap = i64_ty.const_int(8, false);
                            let elem_sz = elem_llvm.size_of().unwrap();

                            // Allocate result vec
                            let alloc_sz = self
                                .builder
                                .build_int_mul(initial_cap, elem_sz, "fil_alloc")
                                .unwrap();
                            let malloc_fn = self.get_or_declare_malloc();
                            let new_data = self
                                .builder
                                .build_call(malloc_fn, &[alloc_sz.into()], "fil_nd")
                                .unwrap()
                                .try_as_basic_value()
                                .basic()
                                .unwrap()
                                .into_pointer_value();
                            let new_vec_ptr = self
                                .builder
                                .build_alloca(vec_struct_ty, "fil_vec")
                                .unwrap();
                            let dg = self.builder.build_struct_gep(vec_struct_ty, new_vec_ptr, 0, "fv_dg").unwrap();
                            self.builder.build_store(dg, new_data).unwrap();
                            let lg = self.builder.build_struct_gep(vec_struct_ty, new_vec_ptr, 1, "fv_lg").unwrap();
                            self.builder.build_store(lg, zero).unwrap();
                            let cg = self.builder.build_struct_gep(vec_struct_ty, new_vec_ptr, 2, "fv_cg").unwrap();
                            self.builder.build_store(cg, initial_cap).unwrap();
                            let eg = self.builder.build_struct_gep(vec_struct_ty, new_vec_ptr, 3, "fv_eg").unwrap();
                            self.builder.build_store(eg, elem_sz).unwrap();

                            // Loop over source
                            let pre_bb = self.builder.get_insert_block().unwrap();
                            let loop_bb = self.context.append_basic_block(*function, "fil_loop");
                            let check_bb = self.context.append_basic_block(*function, "fil_check");
                            let push_bb = self.context.append_basic_block(*function, "fil_push");
                            let grow_bb = self.context.append_basic_block(*function, "fil_grow");
                            let store_bb = self.context.append_basic_block(*function, "fil_store");
                            let skip_bb = self.context.append_basic_block(*function, "fil_skip");
                            let done_bb = self.context.append_basic_block(*function, "fil_done");

                            self.builder.build_unconditional_branch(loop_bb).unwrap();

                            self.builder.position_at_end(loop_bb);
                            let i_phi = self.builder.build_phi(i64_ty, "fil_i").unwrap();
                            i_phi.add_incoming(&[(&zero, pre_bb)]);
                            let i_val = i_phi.as_basic_value().into_int_value();
                            let cond = self.builder
                                .build_int_compare(IntPredicate::ULT, i_val, len, "fil_cond")
                                .unwrap();
                            self.builder.build_conditional_branch(cond, check_bb, done_bb).unwrap();

                            // Call predicate
                            self.builder.position_at_end(check_bb);
                            let ep = unsafe {
                                self.builder.build_in_bounds_gep(elem_llvm, data_ptr, &[i_val], "fil_ep").unwrap()
                            };
                            let ev = self.builder.build_load(elem_llvm, ep, "fil_ev").unwrap();
                            let keep = if let Some((lambda_fn_name, cap_vars)) = &closure_info {
                                if let Some((func, _, _)) = self.functions.get(lambda_fn_name).cloned() {
                                    let mut call_args: Vec<inkwell::values::BasicMetadataValueEnum> = Vec::new();
                                    for (cap_name, cap_ty) in cap_vars {
                                        if let Some((cap_ptr, _)) = self.variables.get(cap_name) {
                                            let cap_llvm = ny_to_llvm(self.context, cap_ty);
                                            let cv = self.builder.build_load(cap_llvm, *cap_ptr, cap_name).unwrap();
                                            call_args.push(cv.into());
                                        }
                                    }
                                    call_args.push(ev.into());
                                    self.builder.build_call(func, &call_args, "fil_keep").unwrap()
                                        .try_as_basic_value().basic().unwrap().into_int_value()
                                } else {
                                    let pred_ty = self.context.bool_type().fn_type(&[elem_llvm.into()], false);
                                    self.builder.build_indirect_call(pred_ty, fn_ptr_val, &[ev.into()], "fil_keep").unwrap()
                                        .try_as_basic_value().basic().unwrap().into_int_value()
                                }
                            } else {
                                let pred_ty = self.context.bool_type().fn_type(&[elem_llvm.into()], false);
                                self.builder.build_indirect_call(pred_ty, fn_ptr_val, &[ev.into()], "fil_keep").unwrap()
                                    .try_as_basic_value().basic().unwrap().into_int_value()
                            };
                            self.builder.build_conditional_branch(keep, push_bb, skip_bb).unwrap();

                            // Push element (with grow check)
                            self.builder.position_at_end(push_bb);
                            let nv_len_gep = self.builder.build_struct_gep(vec_struct_ty, new_vec_ptr, 1, "fpl").unwrap();
                            let nv_cap_gep = self.builder.build_struct_gep(vec_struct_ty, new_vec_ptr, 2, "fpc").unwrap();
                            let nv_data_gep = self.builder.build_struct_gep(vec_struct_ty, new_vec_ptr, 0, "fpd").unwrap();
                            let cur_len = self.builder.build_load(i64_ty, nv_len_gep, "fcl").unwrap().into_int_value();
                            let cur_cap = self.builder.build_load(i64_ty, nv_cap_gep, "fcc").unwrap().into_int_value();
                            let needs_grow = self.builder
                                .build_int_compare(IntPredicate::UGE, cur_len, cur_cap, "fng")
                                .unwrap();
                            self.builder.build_conditional_branch(needs_grow, grow_bb, store_bb).unwrap();

                            self.builder.position_at_end(grow_bb);
                            let nc = self.builder.build_int_mul(cur_cap, i64_ty.const_int(2, false), "fnc").unwrap();
                            let ns = self.builder.build_int_mul(nc, elem_sz, "fns").unwrap();
                            let realloc_fn = self.get_or_declare_realloc();
                            let cd = self.builder.build_load(self.context.ptr_type(AddressSpace::default()), nv_data_gep, "fcd").unwrap().into_pointer_value();
                            let nd = self.builder.build_call(realloc_fn, &[cd.into(), ns.into()], "fnd").unwrap().try_as_basic_value().basic().unwrap().into_pointer_value();
                            self.builder.build_store(nv_data_gep, nd).unwrap();
                            self.builder.build_store(nv_cap_gep, nc).unwrap();
                            self.builder.build_unconditional_branch(store_bb).unwrap();

                            self.builder.position_at_end(store_bb);
                            let sd = self.builder.build_load(self.context.ptr_type(AddressSpace::default()), nv_data_gep, "fsd").unwrap().into_pointer_value();
                            let sl = self.builder.build_load(i64_ty, nv_len_gep, "fsl").unwrap().into_int_value();
                            let dp = unsafe { self.builder.build_in_bounds_gep(elem_llvm, sd, &[sl], "fdp").unwrap() };
                            self.builder.build_store(dp, ev).unwrap();
                            let nl = self.builder.build_int_add(sl, one, "fnl").unwrap();
                            self.builder.build_store(nv_len_gep, nl).unwrap();
                            self.builder.build_unconditional_branch(skip_bb).unwrap();

                            // Skip / next iteration
                            self.builder.position_at_end(skip_bb);
                            let next_i = self.builder.build_int_add(i_val, one, "fil_next").unwrap();
                            i_phi.add_incoming(&[(&next_i, skip_bb)]);
                            self.builder.build_unconditional_branch(loop_bb).unwrap();

                            self.builder.position_at_end(done_bb);
                            let result = self.builder.build_load(vec_struct_ty, new_vec_ptr, "fil_result").unwrap();
                            return Ok(Some(result));
                        }
                        "reduce" => {
                            // v.reduce(fn, init) -> T — fold all elements with binary fn
                            let obj_val = self.compile_expr(object, function)?.unwrap();
                            let sv = obj_val.into_struct_value();
                            let data_ptr = self
                                .builder
                                .build_extract_value(sv, 0, "red_data")
                                .unwrap()
                                .into_pointer_value();
                            let len = self
                                .builder
                                .build_extract_value(sv, 1, "red_len")
                                .unwrap()
                                .into_int_value();
                            let fn_ptr = self
                                .compile_expr(&args[0], function)?
                                .unwrap()
                                .into_pointer_value();
                            let closure_info = if let Expr::Ident { name, .. } = &args[0] {
                                self.closure_captures.get(name).cloned()
                            } else if let Expr::Lambda { .. } = &args[0] {
                                self.closure_captures
                                    .iter()
                                    .filter(|(k, _)| k.starts_with("__lambda_"))
                                    .max_by_key(|(k, _)| k.to_string())
                                    .map(|(_, v)| v.clone())
                            } else {
                                None
                            };
                            let init_val = self
                                .compile_expr(&args[1], function)?
                                .unwrap();

                            let i64_ty = self.context.i64_type();
                            let zero = i64_ty.const_int(0, false);
                            let one = i64_ty.const_int(1, false);

                            // fn(acc: T, elem: T) -> T
                            let fn_ty = elem_llvm.fn_type(
                                &[elem_llvm.into(), elem_llvm.into()],
                                false,
                            );

                            let pre_bb = self.builder.get_insert_block().unwrap();
                            let loop_bb =
                                self.context.append_basic_block(*function, "red_loop");
                            let body_bb =
                                self.context.append_basic_block(*function, "red_body");
                            let done_bb =
                                self.context.append_basic_block(*function, "red_done");

                            self.builder.build_unconditional_branch(loop_bb).unwrap();

                            self.builder.position_at_end(loop_bb);
                            let i_phi =
                                self.builder.build_phi(i64_ty, "red_i").unwrap();
                            i_phi.add_incoming(&[(&zero, pre_bb)]);
                            let acc_phi =
                                self.builder.build_phi(elem_llvm, "red_acc").unwrap();
                            acc_phi.add_incoming(&[(&init_val, pre_bb)]);
                            let i_val = i_phi.as_basic_value().into_int_value();

                            let cond = self
                                .builder
                                .build_int_compare(IntPredicate::ULT, i_val, len, "red_cond")
                                .unwrap();
                            self.builder
                                .build_conditional_branch(cond, body_bb, done_bb)
                                .unwrap();

                            self.builder.position_at_end(body_bb);
                            let ep = unsafe {
                                self.builder
                                    .build_in_bounds_gep(
                                        elem_llvm, data_ptr, &[i_val], "red_ep",
                                    )
                                    .unwrap()
                            };
                            let ev = self
                                .builder
                                .build_load(elem_llvm, ep, "red_ev")
                                .unwrap();
                            let new_acc = if let Some((lambda_fn_name, cap_vars)) = &closure_info {
                                if let Some((func, _, _)) = self.functions.get(lambda_fn_name).cloned() {
                                    let mut call_args: Vec<inkwell::values::BasicMetadataValueEnum> = Vec::new();
                                    for (cap_name, cap_ty) in cap_vars {
                                        if let Some((cap_ptr, _)) = self.variables.get(cap_name) {
                                            let cap_llvm = ny_to_llvm(self.context, cap_ty);
                                            let cv = self.builder.build_load(cap_llvm, *cap_ptr, cap_name).unwrap();
                                            call_args.push(cv.into());
                                        }
                                    }
                                    call_args.push(acc_phi.as_basic_value().into());
                                    call_args.push(ev.into());
                                    self.builder.build_call(func, &call_args, "red_r").unwrap()
                                        .try_as_basic_value().basic().unwrap()
                                } else {
                                    self.builder.build_indirect_call(fn_ty, fn_ptr, &[acc_phi.as_basic_value().into(), ev.into()], "red_r").unwrap()
                                        .try_as_basic_value().basic().unwrap()
                                }
                            } else {
                                self.builder.build_indirect_call(fn_ty, fn_ptr, &[acc_phi.as_basic_value().into(), ev.into()], "red_r").unwrap()
                                    .try_as_basic_value().basic().unwrap()
                            };
                            let next_i = self
                                .builder
                                .build_int_add(i_val, one, "red_next")
                                .unwrap();
                            i_phi.add_incoming(&[(&next_i, body_bb)]);
                            acc_phi.add_incoming(&[(&new_acc, body_bb)]);
                            self.builder
                                .build_unconditional_branch(loop_bb)
                                .unwrap();

                            self.builder.position_at_end(done_bb);
                            return Ok(Some(acc_phi.as_basic_value()));
                        }
                        "for_each" => {
                            // v.for_each(fn) — call fn on each element (side effects)
                            let obj_val = self.compile_expr(object, function)?.unwrap();
                            let sv = obj_val.into_struct_value();
                            let data_ptr = self
                                .builder
                                .build_extract_value(sv, 0, "fe_data")
                                .unwrap()
                                .into_pointer_value();
                            let len = self
                                .builder
                                .build_extract_value(sv, 1, "fe_len")
                                .unwrap()
                                .into_int_value();
                            let fn_ptr = self
                                .compile_expr(&args[0], function)?
                                .unwrap()
                                .into_pointer_value();

                            let i64_ty = self.context.i64_type();
                            let zero = i64_ty.const_int(0, false);
                            let one = i64_ty.const_int(1, false);
                            let void_ty = self.context.void_type();
                            let fn_ty = void_ty.fn_type(&[elem_llvm.into()], false);

                            let pre_bb = self.builder.get_insert_block().unwrap();
                            let loop_bb =
                                self.context.append_basic_block(*function, "fe_loop");
                            let body_bb =
                                self.context.append_basic_block(*function, "fe_body");
                            let done_bb =
                                self.context.append_basic_block(*function, "fe_done");

                            self.builder.build_unconditional_branch(loop_bb).unwrap();

                            self.builder.position_at_end(loop_bb);
                            let i_phi =
                                self.builder.build_phi(i64_ty, "fe_i").unwrap();
                            i_phi.add_incoming(&[(&zero, pre_bb)]);
                            let i_val = i_phi.as_basic_value().into_int_value();
                            let cond = self
                                .builder
                                .build_int_compare(IntPredicate::ULT, i_val, len, "fe_cond")
                                .unwrap();
                            self.builder
                                .build_conditional_branch(cond, body_bb, done_bb)
                                .unwrap();

                            self.builder.position_at_end(body_bb);
                            let ep = unsafe {
                                self.builder
                                    .build_in_bounds_gep(
                                        elem_llvm, data_ptr, &[i_val], "fe_ep",
                                    )
                                    .unwrap()
                            };
                            let ev = self
                                .builder
                                .build_load(elem_llvm, ep, "fe_ev")
                                .unwrap();
                            self.builder
                                .build_indirect_call(fn_ty, fn_ptr, &[ev.into()], "")
                                .unwrap();
                            let next_i = self
                                .builder
                                .build_int_add(i_val, one, "fe_next")
                                .unwrap();
                            i_phi.add_incoming(&[(&next_i, body_bb)]);
                            self.builder
                                .build_unconditional_branch(loop_bb)
                                .unwrap();

                            self.builder.position_at_end(done_bb);
                            return Ok(None);
                        }
                        "any" | "all" => {
                            // v.any(pred) -> bool / v.all(pred) -> bool
                            let is_any = method == "any";
                            let obj_val = self.compile_expr(object, function)?.unwrap();
                            let sv = obj_val.into_struct_value();
                            let data_ptr = self
                                .builder
                                .build_extract_value(sv, 0, "qa_data")
                                .unwrap()
                                .into_pointer_value();
                            let len = self
                                .builder
                                .build_extract_value(sv, 1, "qa_len")
                                .unwrap()
                                .into_int_value();
                            let fn_ptr = self
                                .compile_expr(&args[0], function)?
                                .unwrap()
                                .into_pointer_value();

                            let i64_ty = self.context.i64_type();
                            let zero = i64_ty.const_int(0, false);
                            let one = i64_ty.const_int(1, false);
                            let bool_ty = self.context.bool_type();
                            let pred_ty = bool_ty.fn_type(&[elem_llvm.into()], false);

                            let pre_bb = self.builder.get_insert_block().unwrap();
                            let loop_bb = self.context.append_basic_block(*function, "qa_loop");
                            let check_bb = self.context.append_basic_block(*function, "qa_check");
                            let inc_bb = self.context.append_basic_block(*function, "qa_inc");
                            let early_bb = self.context.append_basic_block(*function, "qa_early");
                            let done_bb = self.context.append_basic_block(*function, "qa_done");
                            let merge_bb = self.context.append_basic_block(*function, "qa_merge");

                            self.builder.build_unconditional_branch(loop_bb).unwrap();

                            self.builder.position_at_end(loop_bb);
                            let i_phi = self.builder.build_phi(i64_ty, "qa_i").unwrap();
                            i_phi.add_incoming(&[(&zero, pre_bb)]);
                            let i_val = i_phi.as_basic_value().into_int_value();
                            let cond = self.builder
                                .build_int_compare(IntPredicate::ULT, i_val, len, "qa_cond")
                                .unwrap();
                            self.builder
                                .build_conditional_branch(cond, check_bb, done_bb)
                                .unwrap();

                            self.builder.position_at_end(check_bb);
                            let ep = unsafe {
                                self.builder
                                    .build_in_bounds_gep(elem_llvm, data_ptr, &[i_val], "qa_ep")
                                    .unwrap()
                            };
                            let ev = self.builder
                                .build_load(elem_llvm, ep, "qa_ev")
                                .unwrap();
                            let result = self.builder
                                .build_indirect_call(pred_ty, fn_ptr, &[ev.into()], "qa_r")
                                .unwrap()
                                .try_as_basic_value()
                                .basic()
                                .unwrap()
                                .into_int_value();

                            if is_any {
                                self.builder
                                    .build_conditional_branch(result, early_bb, inc_bb)
                                    .unwrap();
                            } else {
                                self.builder
                                    .build_conditional_branch(result, inc_bb, early_bb)
                                    .unwrap();
                            }

                            // Increment block
                            self.builder.position_at_end(inc_bb);
                            let next_i = self.builder
                                .build_int_add(i_val, one, "qa_next")
                                .unwrap();
                            i_phi.add_incoming(&[(&next_i, inc_bb)]);
                            self.builder.build_unconditional_branch(loop_bb).unwrap();

                            // Early exit: any→true, all→false
                            self.builder.position_at_end(early_bb);
                            let early_val = bool_ty.const_int(if is_any { 1 } else { 0 }, false);
                            self.builder.build_unconditional_branch(merge_bb).unwrap();

                            // Loop exhausted: any→false, all→true
                            self.builder.position_at_end(done_bb);
                            let done_val = bool_ty.const_int(if is_any { 0 } else { 1 }, false);
                            self.builder.build_unconditional_branch(merge_bb).unwrap();

                            self.builder.position_at_end(merge_bb);
                            let phi = self.builder
                                .build_phi(bool_ty, "qa_result")
                                .unwrap();
                            phi.add_incoming(&[(&early_val, early_bb), (&done_val, done_bb)]);
                            return Ok(Some(phi.as_basic_value()));
                        }
                        "join" => {
                            // Vec<str>.join(sep) -> str
                            let obj_val = self.compile_expr(object, function)?.unwrap();
                            let sv = obj_val.into_struct_value();
                            let data_ptr = self
                                .builder
                                .build_extract_value(sv, 0, "join_data")
                                .unwrap()
                                .into_pointer_value();
                            let len = self
                                .builder
                                .build_extract_value(sv, 1, "join_len")
                                .unwrap()
                                .into_int_value();
                            let sep_val = self
                                .compile_expr(&args[0], function)?
                                .unwrap()
                                .into_struct_value();
                            let sep_ptr = self
                                .builder
                                .build_extract_value(sep_val, 0, "join_sp")
                                .unwrap();
                            let sep_len = self
                                .builder
                                .build_extract_value(sep_val, 1, "join_sl")
                                .unwrap();
                            let i64_ty = self.context.i64_type();
                            let out_len_ptr = self
                                .builder
                                .build_alloca(i64_ty, "join_olp")
                                .unwrap();
                            let join_fn = self.get_or_declare_ny_str_join();
                            let result_ptr = self
                                .builder
                                .build_call(
                                    join_fn,
                                    &[
                                        data_ptr.into(),
                                        len.into(),
                                        sep_ptr.into(),
                                        sep_len.into(),
                                        out_len_ptr.into(),
                                    ],
                                    "join_r",
                                )
                                .unwrap()
                                .try_as_basic_value()
                                .basic()
                                .unwrap()
                                .into_pointer_value();
                            let out_len = self
                                .builder
                                .build_load(i64_ty, out_len_ptr, "join_rl")
                                .unwrap()
                                .into_int_value();
                            let str_ty = str_type(self.context);
                            let result = str_ty.const_zero();
                            let result = self
                                .builder
                                .build_insert_value(result, result_ptr, 0, "join_rp")
                                .unwrap();
                            let result = self
                                .builder
                                .build_insert_value(result, out_len, 1, "join_rl2")
                                .unwrap();
                            return Ok(Some(result.into_struct_value().into()));
                        }
                        "sum" => {
                            // v.sum() -> T — sum all elements
                            let obj_val = self.compile_expr(object, function)?.unwrap();
                            let sv = obj_val.into_struct_value();
                            let data_ptr = self
                                .builder
                                .build_extract_value(sv, 0, "sum_data")
                                .unwrap()
                                .into_pointer_value();
                            let len = self
                                .builder
                                .build_extract_value(sv, 1, "sum_len")
                                .unwrap()
                                .into_int_value();

                            let i64_ty = self.context.i64_type();
                            let zero_i64 = i64_ty.const_int(0, false);
                            let one = i64_ty.const_int(1, false);
                            let zero_elem: inkwell::values::BasicValueEnum = if elem_ty.is_float() {
                                elem_llvm.into_float_type().const_float(0.0).into()
                            } else {
                                elem_llvm.into_int_type().const_int(0, false).into()
                            };

                            let pre_bb = self.builder.get_insert_block().unwrap();
                            let loop_bb = self.context.append_basic_block(*function, "sum_loop");
                            let body_bb = self.context.append_basic_block(*function, "sum_body");
                            let done_bb = self.context.append_basic_block(*function, "sum_done");

                            self.builder.build_unconditional_branch(loop_bb).unwrap();

                            self.builder.position_at_end(loop_bb);
                            let i_phi = self.builder.build_phi(i64_ty, "sum_i").unwrap();
                            i_phi.add_incoming(&[(&zero_i64, pre_bb)]);
                            let acc_phi = self.builder.build_phi(elem_llvm, "sum_acc").unwrap();
                            acc_phi.add_incoming(&[(&zero_elem, pre_bb)]);
                            let i_val = i_phi.as_basic_value().into_int_value();

                            let cond = self.builder
                                .build_int_compare(IntPredicate::ULT, i_val, len, "sum_cond")
                                .unwrap();
                            self.builder.build_conditional_branch(cond, body_bb, done_bb).unwrap();

                            self.builder.position_at_end(body_bb);
                            let ep = unsafe {
                                self.builder.build_in_bounds_gep(elem_llvm, data_ptr, &[i_val], "sum_ep").unwrap()
                            };
                            let ev = self.builder.build_load(elem_llvm, ep, "sum_ev").unwrap();
                            let new_acc: inkwell::values::BasicValueEnum = if elem_ty.is_float() {
                                self.builder.build_float_add(
                                    acc_phi.as_basic_value().into_float_value(),
                                    ev.into_float_value(), "sum_fa",
                                ).unwrap().into()
                            } else {
                                self.builder.build_int_add(
                                    acc_phi.as_basic_value().into_int_value(),
                                    ev.into_int_value(), "sum_ia",
                                ).unwrap().into()
                            };
                            let next_i = self.builder.build_int_add(i_val, one, "sum_next").unwrap();
                            i_phi.add_incoming(&[(&next_i, body_bb)]);
                            acc_phi.add_incoming(&[(&new_acc, body_bb)]);
                            self.builder.build_unconditional_branch(loop_bb).unwrap();

                            self.builder.position_at_end(done_bb);
                            return Ok(Some(acc_phi.as_basic_value()));
                        }
                        "contains" | "index_of" => {
                            // v.contains(val) -> bool / v.index_of(val) -> i32
                            let obj_val = self.compile_expr(object, function)?.unwrap();
                            let sv = obj_val.into_struct_value();
                            let data_ptr = self
                                .builder
                                .build_extract_value(sv, 0, "vc_data")
                                .unwrap()
                                .into_pointer_value();
                            let len = self
                                .builder
                                .build_extract_value(sv, 1, "vc_len")
                                .unwrap()
                                .into_int_value();
                            let needle = self
                                .compile_expr(&args[0], function)?
                                .unwrap();

                            let i64_ty = self.context.i64_type();
                            let i32_ty = self.context.i32_type();
                            let zero = i64_ty.const_int(0, false);
                            let one = i64_ty.const_int(1, false);

                            let pre_bb = self.builder.get_insert_block().unwrap();
                            let loop_bb =
                                self.context.append_basic_block(*function, "vc_loop");
                            let check_bb =
                                self.context.append_basic_block(*function, "vc_check");
                            let inc_bb =
                                self.context.append_basic_block(*function, "vc_inc");
                            let found_bb =
                                self.context.append_basic_block(*function, "vc_found");
                            let not_bb =
                                self.context.append_basic_block(*function, "vc_not");
                            let merge_bb =
                                self.context.append_basic_block(*function, "vc_merge");

                            self.builder.build_unconditional_branch(loop_bb).unwrap();

                            self.builder.position_at_end(loop_bb);
                            let i_phi =
                                self.builder.build_phi(i64_ty, "vc_i").unwrap();
                            i_phi.add_incoming(&[(&zero, pre_bb)]);
                            let i_val = i_phi.as_basic_value().into_int_value();
                            let cond = self
                                .builder
                                .build_int_compare(IntPredicate::ULT, i_val, len, "vc_cond")
                                .unwrap();
                            self.builder
                                .build_conditional_branch(cond, check_bb, not_bb)
                                .unwrap();

                            self.builder.position_at_end(check_bb);
                            let elem_ptr = unsafe {
                                self.builder
                                    .build_in_bounds_gep(
                                        elem_llvm, data_ptr, &[i_val], "vc_ptr",
                                    )
                                    .unwrap()
                            };
                            let elem_val = self
                                .builder
                                .build_load(elem_llvm, elem_ptr, "vc_val")
                                .unwrap();

                            let is_eq = if elem_ty.is_float() {
                                self.builder
                                    .build_float_compare(
                                        inkwell::FloatPredicate::OEQ,
                                        elem_val.into_float_value(),
                                        needle.into_float_value(),
                                        "vc_feq",
                                    )
                                    .unwrap()
                            } else {
                                self.builder
                                    .build_int_compare(
                                        IntPredicate::EQ,
                                        elem_val.into_int_value(),
                                        needle.into_int_value(),
                                        "vc_ieq",
                                    )
                                    .unwrap()
                            };
                            self.builder
                                .build_conditional_branch(is_eq, found_bb, inc_bb)
                                .unwrap();

                            self.builder.position_at_end(inc_bb);
                            let next_i = self
                                .builder
                                .build_int_add(i_val, one, "vc_next")
                                .unwrap();
                            i_phi.add_incoming(&[(&next_i, inc_bb)]);
                            self.builder
                                .build_unconditional_branch(loop_bb)
                                .unwrap();

                            if method == "contains" {
                                // contains: return bool
                                self.builder.position_at_end(found_bb);
                                let true_val =
                                    self.context.bool_type().const_int(1, false);
                                self.builder
                                    .build_unconditional_branch(merge_bb)
                                    .unwrap();

                                self.builder.position_at_end(not_bb);
                                let false_val =
                                    self.context.bool_type().const_int(0, false);
                                self.builder
                                    .build_unconditional_branch(merge_bb)
                                    .unwrap();

                                self.builder.position_at_end(merge_bb);
                                let phi = self
                                    .builder
                                    .build_phi(self.context.bool_type(), "vc_result")
                                    .unwrap();
                                phi.add_incoming(&[
                                    (&true_val, found_bb),
                                    (&false_val, not_bb),
                                ]);
                                return Ok(Some(phi.as_basic_value()));
                            } else {
                                // index_of: return i32
                                self.builder.position_at_end(found_bb);
                                let idx = self
                                    .builder
                                    .build_int_truncate(i_val, i32_ty, "vc_idx")
                                    .unwrap();
                                self.builder
                                    .build_unconditional_branch(merge_bb)
                                    .unwrap();

                                self.builder.position_at_end(not_bb);
                                let neg1 = i32_ty.const_all_ones();
                                self.builder
                                    .build_unconditional_branch(merge_bb)
                                    .unwrap();

                                self.builder.position_at_end(merge_bb);
                                let phi = self
                                    .builder
                                    .build_phi(i32_ty, "vc_idx_result")
                                    .unwrap();
                                phi.add_incoming(&[
                                    (&idx, found_bb),
                                    (&neg1, not_bb),
                                ]);
                                return Ok(Some(phi.as_basic_value()));
                            }
                        }
                        _ => {}
                    }
                }

                // Handle built-in slice methods
                if let NyType::Slice(_) = &obj_ty {
                    let obj_val = self.compile_expr(object, function)?.unwrap();
                    let slice_val = obj_val.into_struct_value();
                    if method.as_str() == "len" {
                        let len_val = self
                            .builder
                            .build_extract_value(slice_val, 1, "slice_len")
                            .unwrap();
                        return Ok(Some(len_val));
                    }
                }

                // Handle built-in string methods
                if obj_ty == NyType::Str {
                    let obj_val = self.compile_expr(object, function)?.unwrap();
                    let str_val = obj_val.into_struct_value();

                    match method.as_str() {
                        "len" => {
                            // Extract length field (index 1) from {ptr, len}
                            let len_val = self
                                .builder
                                .build_extract_value(str_val, 1, "str_len")
                                .unwrap();
                            return Ok(Some(len_val));
                        }
                        "substr" => {
                            // substr(start, end) -> {new_ptr, new_len}
                            let start_val = self
                                .compile_expr(&args[0], function)?
                                .unwrap()
                                .into_int_value();
                            let end_val = self
                                .compile_expr(&args[1], function)?
                                .unwrap()
                                .into_int_value();

                            let ptr = self
                                .builder
                                .build_extract_value(str_val, 0, "str_ptr")
                                .unwrap()
                                .into_pointer_value();
                            let str_len = self
                                .builder
                                .build_extract_value(str_val, 1, "sub_str_len")
                                .unwrap()
                                .into_int_value();

                            // Cast start to i64 for GEP
                            let start_i64 = self
                                .builder
                                .build_int_z_extend_or_bit_cast(
                                    start_val,
                                    self.context.i64_type(),
                                    "start_ext",
                                )
                                .unwrap();
                            let end_i64 = self
                                .builder
                                .build_int_z_extend_or_bit_cast(
                                    end_val,
                                    self.context.i64_type(),
                                    "end_ext",
                                )
                                .unwrap();

                            // Bounds check: end <= str_len (uses < with len+1)
                            let len_plus1 = self
                                .builder
                                .build_int_add(
                                    str_len,
                                    self.context.i64_type().const_int(1, false),
                                    "sub_lp1",
                                )
                                .unwrap();
                            self.build_bounds_check(end_i64, len_plus1, function);

                            // new_ptr = GEP(ptr, start)
                            let i8_ty = self.context.i8_type();
                            let new_ptr = unsafe {
                                self.builder
                                    .build_in_bounds_gep(i8_ty, ptr, &[start_i64], "substr_ptr")
                                    .unwrap()
                            };

                            // new_len = end - start
                            let new_len = self
                                .builder
                                .build_int_sub(end_i64, start_i64, "substr_len")
                                .unwrap();

                            // Build {new_ptr, new_len}
                            let str_ty = str_type(self.context);
                            let result = str_ty.const_zero();
                            let result = self
                                .builder
                                .build_insert_value(result, new_ptr, 0, "sub_ptr")
                                .unwrap();
                            let result = self
                                .builder
                                .build_insert_value(result, new_len, 1, "sub_len")
                                .unwrap();
                            return Ok(Some(result.into_struct_value().into()));
                        }
                        "char_at" => {
                            // s.char_at(index) -> i32 (byte value as int)
                            let idx_val = self
                                .compile_expr(&args[0], function)?
                                .unwrap()
                                .into_int_value();
                            let ptr = self
                                .builder
                                .build_extract_value(str_val, 0, "ca_ptr")
                                .unwrap()
                                .into_pointer_value();
                            let len = self
                                .builder
                                .build_extract_value(str_val, 1, "ca_len")
                                .unwrap()
                                .into_int_value();
                            let idx_i64 = self
                                .builder
                                .build_int_z_extend_or_bit_cast(
                                    idx_val,
                                    self.context.i64_type(),
                                    "ca_idx64",
                                )
                                .unwrap();
                            self.build_bounds_check(idx_i64, len, function);
                            let i8_ty = self.context.i8_type();
                            let char_ptr = unsafe {
                                self.builder
                                    .build_in_bounds_gep(i8_ty, ptr, &[idx_i64], "char_ptr")
                                    .unwrap()
                            };
                            let byte = self
                                .builder
                                .build_load(i8_ty, char_ptr, "char_byte")
                                .unwrap()
                                .into_int_value();
                            let as_i32 = self
                                .builder
                                .build_int_z_extend(byte, self.context.i32_type(), "char_i32")
                                .unwrap();
                            return Ok(Some(as_i32.into()));
                        }
                        "trim" => {
                            // s.trim() -> str — strip leading/trailing whitespace (no alloc)
                            let ptr = self
                                .builder
                                .build_extract_value(str_val, 0, "trim_ptr")
                                .unwrap()
                                .into_pointer_value();
                            let len = self
                                .builder
                                .build_extract_value(str_val, 1, "trim_len")
                                .unwrap()
                                .into_int_value();

                            let i8_ty = self.context.i8_type();
                            let i64_ty = self.context.i64_type();
                            let zero = i64_ty.const_int(0, false);
                            let one = i64_ty.const_int(1, false);
                            let space = i8_ty.const_int(32, false); // ' '

                            // Find start: skip leading spaces/tabs/newlines
                            let pre_bb = self.builder.get_insert_block().unwrap();
                            let start_loop =
                                self.context.append_basic_block(*function, "trim_start");
                            let start_done =
                                self.context.append_basic_block(*function, "trim_start_done");
                            self.builder.build_unconditional_branch(start_loop).unwrap();

                            self.builder.position_at_end(start_loop);
                            let s_phi = self.builder.build_phi(i64_ty, "trim_s").unwrap();
                            s_phi.add_incoming(&[(&zero, pre_bb)]);
                            let s_val = s_phi.as_basic_value().into_int_value();
                            let in_range = self
                                .builder
                                .build_int_compare(IntPredicate::ULT, s_val, len, "s_lt")
                                .unwrap();
                            let check_bb =
                                self.context.append_basic_block(*function, "trim_s_check");
                            self.builder
                                .build_conditional_branch(in_range, check_bb, start_done)
                                .unwrap();

                            self.builder.position_at_end(check_bb);
                            let ch_ptr = unsafe {
                                self.builder
                                    .build_in_bounds_gep(i8_ty, ptr, &[s_val], "s_ch_ptr")
                                    .unwrap()
                            };
                            let ch = self
                                .builder
                                .build_load(i8_ty, ch_ptr, "s_ch")
                                .unwrap()
                                .into_int_value();
                            let is_space = self
                                .builder
                                .build_int_compare(IntPredicate::ULE, ch, space, "is_sp")
                                .unwrap();
                            let next_s = self
                                .builder
                                .build_int_add(s_val, one, "next_s")
                                .unwrap();
                            s_phi.add_incoming(&[(&next_s, check_bb)]);
                            self.builder
                                .build_conditional_branch(is_space, start_loop, start_done)
                                .unwrap();

                            self.builder.position_at_end(start_done);
                            let start_idx = self
                                .builder
                                .build_phi(i64_ty, "start_idx")
                                .unwrap();
                            start_idx.add_incoming(&[(&s_val, start_loop), (&s_val, check_bb)]);
                            let start_val = start_idx.as_basic_value().into_int_value();

                            // Find end: skip trailing spaces
                            let end_pre = self.builder.get_insert_block().unwrap();
                            let end_loop =
                                self.context.append_basic_block(*function, "trim_end");
                            let end_done =
                                self.context.append_basic_block(*function, "trim_end_done");
                            self.builder.build_unconditional_branch(end_loop).unwrap();

                            self.builder.position_at_end(end_loop);
                            let e_phi = self.builder.build_phi(i64_ty, "trim_e").unwrap();
                            e_phi.add_incoming(&[(&len, end_pre)]);
                            let e_val = e_phi.as_basic_value().into_int_value();
                            let e_gt_start = self
                                .builder
                                .build_int_compare(IntPredicate::UGT, e_val, start_val, "e_gt")
                                .unwrap();
                            let end_check_bb =
                                self.context.append_basic_block(*function, "trim_e_check");
                            self.builder
                                .build_conditional_branch(e_gt_start, end_check_bb, end_done)
                                .unwrap();

                            self.builder.position_at_end(end_check_bb);
                            let e_m1 = self
                                .builder
                                .build_int_sub(e_val, one, "e_m1")
                                .unwrap();
                            let ech_ptr = unsafe {
                                self.builder
                                    .build_in_bounds_gep(i8_ty, ptr, &[e_m1], "e_ch_ptr")
                                    .unwrap()
                            };
                            let ech = self
                                .builder
                                .build_load(i8_ty, ech_ptr, "e_ch")
                                .unwrap()
                                .into_int_value();
                            let e_is_space = self
                                .builder
                                .build_int_compare(IntPredicate::ULE, ech, space, "e_is_sp")
                                .unwrap();
                            e_phi.add_incoming(&[(&e_m1, end_check_bb)]);
                            self.builder
                                .build_conditional_branch(e_is_space, end_loop, end_done)
                                .unwrap();

                            self.builder.position_at_end(end_done);
                            let end_idx = self
                                .builder
                                .build_phi(i64_ty, "end_idx")
                                .unwrap();
                            end_idx.add_incoming(&[(&e_val, end_loop), (&e_val, end_check_bb)]);
                            let end_val = end_idx.as_basic_value().into_int_value();

                            // Build result: {ptr + start, end - start}
                            let new_ptr = unsafe {
                                self.builder
                                    .build_in_bounds_gep(
                                        i8_ty, ptr, &[start_val], "trim_new_ptr",
                                    )
                                    .unwrap()
                            };
                            let new_len = self
                                .builder
                                .build_int_sub(end_val, start_val, "trim_new_len")
                                .unwrap();

                            let str_ty = str_type(self.context);
                            let result = str_ty.const_zero();
                            let result = self
                                .builder
                                .build_insert_value(result, new_ptr, 0, "t_ptr")
                                .unwrap();
                            let result = self
                                .builder
                                .build_insert_value(result, new_len, 1, "t_len")
                                .unwrap();
                            return Ok(Some(result.into_struct_value().into()));
                        }
                        "to_upper" | "to_lower" => {
                            // Allocate new string, convert each byte
                            let ptr = self
                                .builder
                                .build_extract_value(str_val, 0, "case_ptr")
                                .unwrap()
                                .into_pointer_value();
                            let len = self
                                .builder
                                .build_extract_value(str_val, 1, "case_len")
                                .unwrap()
                                .into_int_value();

                            let malloc_fn = self.get_or_declare_malloc();
                            let new_buf = self
                                .builder
                                .build_call(malloc_fn, &[len.into()], "case_buf")
                                .unwrap()
                                .try_as_basic_value()
                                .basic()
                                .unwrap()
                                .into_pointer_value();

                            let convert_fn = if method == "to_upper" {
                                self.get_or_declare_toupper()
                            } else {
                                self.get_or_declare_tolower()
                            };

                            let i8_ty = self.context.i8_type();
                            let i32_ty = self.context.i32_type();
                            let i64_ty = self.context.i64_type();
                            let zero = i64_ty.const_int(0, false);
                            let one = i64_ty.const_int(1, false);

                            let pre_bb = self.builder.get_insert_block().unwrap();
                            let loop_bb =
                                self.context.append_basic_block(*function, "case_loop");
                            let body_bb =
                                self.context.append_basic_block(*function, "case_body");
                            let done_bb =
                                self.context.append_basic_block(*function, "case_done");

                            self.builder.build_unconditional_branch(loop_bb).unwrap();

                            self.builder.position_at_end(loop_bb);
                            let i_phi =
                                self.builder.build_phi(i64_ty, "case_i").unwrap();
                            i_phi.add_incoming(&[(&zero, pre_bb)]);
                            let i_val = i_phi.as_basic_value().into_int_value();
                            let cond = self
                                .builder
                                .build_int_compare(IntPredicate::ULT, i_val, len, "case_cond")
                                .unwrap();
                            self.builder
                                .build_conditional_branch(cond, body_bb, done_bb)
                                .unwrap();

                            self.builder.position_at_end(body_bb);
                            let src_ptr = unsafe {
                                self.builder
                                    .build_in_bounds_gep(i8_ty, ptr, &[i_val], "src_p")
                                    .unwrap()
                            };
                            let byte = self
                                .builder
                                .build_load(i8_ty, src_ptr, "src_byte")
                                .unwrap()
                                .into_int_value();
                            let byte_i32 = self
                                .builder
                                .build_int_z_extend(byte, i32_ty, "byte_i32")
                                .unwrap();
                            let converted = self
                                .builder
                                .build_call(convert_fn, &[byte_i32.into()], "conv")
                                .unwrap()
                                .try_as_basic_value()
                                .basic()
                                .unwrap()
                                .into_int_value();
                            let conv_i8 = self
                                .builder
                                .build_int_truncate(converted, i8_ty, "conv_i8")
                                .unwrap();
                            let dst_ptr = unsafe {
                                self.builder
                                    .build_in_bounds_gep(i8_ty, new_buf, &[i_val], "dst_p")
                                    .unwrap()
                            };
                            self.builder.build_store(dst_ptr, conv_i8).unwrap();
                            let next_i = self
                                .builder
                                .build_int_add(i_val, one, "case_next")
                                .unwrap();
                            i_phi.add_incoming(&[(&next_i, body_bb)]);
                            self.builder
                                .build_unconditional_branch(loop_bb)
                                .unwrap();

                            self.builder.position_at_end(done_bb);
                            let str_ty = str_type(self.context);
                            let result = str_ty.const_zero();
                            let result = self
                                .builder
                                .build_insert_value(result, new_buf, 0, "cu_ptr")
                                .unwrap();
                            let result = self
                                .builder
                                .build_insert_value(result, len, 1, "cu_len")
                                .unwrap();
                            return Ok(Some(result.into_struct_value().into()));
                        }
                        "replace" => {
                            // s.replace(old, new) -> str — returns new heap-allocated string
                            let ptr = self
                                .builder
                                .build_extract_value(str_val, 0, "rep_ptr")
                                .unwrap()
                                .into_pointer_value();
                            let len = self
                                .builder
                                .build_extract_value(str_val, 1, "rep_len")
                                .unwrap()
                                .into_int_value();

                            let old_val = self
                                .compile_expr(&args[0], function)?
                                .unwrap()
                                .into_struct_value();
                            let new_val = self
                                .compile_expr(&args[1], function)?
                                .unwrap()
                                .into_struct_value();

                            let old_ptr = self
                                .builder
                                .build_extract_value(old_val, 0, "old_ptr")
                                .unwrap()
                                .into_pointer_value();
                            let old_len = self
                                .builder
                                .build_extract_value(old_val, 1, "old_len")
                                .unwrap()
                                .into_int_value();
                            let new_ptr = self
                                .builder
                                .build_extract_value(new_val, 0, "new_ptr")
                                .unwrap()
                                .into_pointer_value();
                            let new_len = self
                                .builder
                                .build_extract_value(new_val, 1, "new_len")
                                .unwrap()
                                .into_int_value();

                            // Alloca for out_len
                            let i64_ty = self.context.i64_type();
                            let out_len_ptr = self
                                .builder
                                .build_alloca(i64_ty, "rep_out_len")
                                .unwrap();

                            let replace_fn = self.get_or_declare_ny_str_replace();
                            let result_ptr = self
                                .builder
                                .build_call(
                                    replace_fn,
                                    &[
                                        ptr.into(),
                                        len.into(),
                                        old_ptr.into(),
                                        old_len.into(),
                                        new_ptr.into(),
                                        new_len.into(),
                                        out_len_ptr.into(),
                                    ],
                                    "rep_result",
                                )
                                .unwrap()
                                .try_as_basic_value()
                                .basic()
                                .unwrap()
                                .into_pointer_value();

                            let result_len = self
                                .builder
                                .build_load(i64_ty, out_len_ptr, "rep_rlen")
                                .unwrap()
                                .into_int_value();

                            let str_ty = str_type(self.context);
                            let result = str_ty.const_zero();
                            let result = self
                                .builder
                                .build_insert_value(result, result_ptr, 0, "rep_s_ptr")
                                .unwrap();
                            let result = self
                                .builder
                                .build_insert_value(result, result_len, 1, "rep_s_len")
                                .unwrap();
                            return Ok(Some(result.into_struct_value().into()));
                        }
                        "repeat" => {
                            // s.repeat(n) -> str — repeat string n times
                            let ptr = self
                                .builder
                                .build_extract_value(str_val, 0, "rep_ptr")
                                .unwrap()
                                .into_pointer_value();
                            let len = self
                                .builder
                                .build_extract_value(str_val, 1, "rep_len")
                                .unwrap()
                                .into_int_value();
                            let n_val = self
                                .compile_expr(&args[0], function)?
                                .unwrap()
                                .into_int_value();
                            let n_i64 = self
                                .builder
                                .build_int_z_extend_or_bit_cast(
                                    n_val,
                                    self.context.i64_type(),
                                    "rep_n64",
                                )
                                .unwrap();

                            let total_len = self
                                .builder
                                .build_int_mul(len, n_i64, "rep_total")
                                .unwrap();

                            let malloc_fn = self.get_or_declare_malloc();
                            let buf = self
                                .builder
                                .build_call(malloc_fn, &[total_len.into()], "rep_buf")
                                .unwrap()
                                .try_as_basic_value()
                                .basic()
                                .unwrap()
                                .into_pointer_value();

                            let memcpy_fn = self.get_or_declare_memcpy();
                            let i8_ty = self.context.i8_type();
                            let i64_ty = self.context.i64_type();
                            let zero = i64_ty.const_int(0, false);
                            let one = i64_ty.const_int(1, false);

                            let pre_bb = self.builder.get_insert_block().unwrap();
                            let loop_bb =
                                self.context.append_basic_block(*function, "rep_loop");
                            let body_bb =
                                self.context.append_basic_block(*function, "rep_body");
                            let done_bb =
                                self.context.append_basic_block(*function, "rep_done");

                            self.builder.build_unconditional_branch(loop_bb).unwrap();

                            self.builder.position_at_end(loop_bb);
                            let i_phi =
                                self.builder.build_phi(i64_ty, "rep_i").unwrap();
                            i_phi.add_incoming(&[(&zero, pre_bb)]);
                            let i_val = i_phi.as_basic_value().into_int_value();
                            let cond = self
                                .builder
                                .build_int_compare(IntPredicate::ULT, i_val, n_i64, "rep_cond")
                                .unwrap();
                            self.builder
                                .build_conditional_branch(cond, body_bb, done_bb)
                                .unwrap();

                            self.builder.position_at_end(body_bb);
                            let offset = self
                                .builder
                                .build_int_mul(i_val, len, "rep_off")
                                .unwrap();
                            let dst = unsafe {
                                self.builder
                                    .build_in_bounds_gep(i8_ty, buf, &[offset], "rep_dst")
                                    .unwrap()
                            };
                            self.builder
                                .build_call(
                                    memcpy_fn,
                                    &[dst.into(), ptr.into(), len.into()],
                                    "",
                                )
                                .unwrap();
                            let next_i = self
                                .builder
                                .build_int_add(i_val, one, "rep_next")
                                .unwrap();
                            i_phi.add_incoming(&[(&next_i, body_bb)]);
                            self.builder
                                .build_unconditional_branch(loop_bb)
                                .unwrap();

                            self.builder.position_at_end(done_bb);
                            let str_ty = str_type(self.context);
                            let result = str_ty.const_zero();
                            let result = self
                                .builder
                                .build_insert_value(result, buf, 0, "rep_sp")
                                .unwrap();
                            let result = self
                                .builder
                                .build_insert_value(result, total_len, 1, "rep_sl")
                                .unwrap();
                            return Ok(Some(result.into_struct_value().into()));
                        }
                        "index_of" => {
                            // s.index_of(needle) -> i32 (-1 if not found)
                            let needle_val = self
                                .compile_expr(&args[0], function)?
                                .unwrap()
                                .into_struct_value();

                            let hay_ptr = self
                                .builder
                                .build_extract_value(str_val, 0, "io_hay_ptr")
                                .unwrap()
                                .into_pointer_value();
                            let hay_len = self
                                .builder
                                .build_extract_value(str_val, 1, "io_hay_len")
                                .unwrap()
                                .into_int_value();
                            let ndl_ptr = self
                                .builder
                                .build_extract_value(needle_val, 0, "io_ndl_ptr")
                                .unwrap()
                                .into_pointer_value();
                            let ndl_len = self
                                .builder
                                .build_extract_value(needle_val, 1, "io_ndl_len")
                                .unwrap()
                                .into_int_value();

                            let memcmp_fn = self.get_or_declare_memcmp();
                            let i8_ty = self.context.i8_type();
                            let i32_ty = self.context.i32_type();
                            let i64_ty = self.context.i64_type();
                            let zero_i64 = i64_ty.const_int(0, false);
                            let one_i64 = i64_ty.const_int(1, false);
                            let zero_i32 = i32_ty.const_int(0, false);
                            let neg1_i32 = i32_ty.const_all_ones(); // -1

                            let io_pre_bb = self.builder.get_insert_block().unwrap();
                            let io_loop_bb =
                                self.context.append_basic_block(*function, "io_loop");
                            let io_check_bb =
                                self.context.append_basic_block(*function, "io_check");
                            let io_inc_bb =
                                self.context.append_basic_block(*function, "io_inc");
                            let io_found_bb =
                                self.context.append_basic_block(*function, "io_found");
                            let io_not_bb =
                                self.context.append_basic_block(*function, "io_not");
                            let io_merge_bb =
                                self.context.append_basic_block(*function, "io_merge");

                            let len_ok = self
                                .builder
                                .build_int_compare(
                                    IntPredicate::ULE, ndl_len, hay_len, "io_len_ok",
                                )
                                .unwrap();
                            self.builder
                                .build_conditional_branch(len_ok, io_loop_bb, io_not_bb)
                                .unwrap();

                            self.builder.position_at_end(io_loop_bb);
                            let i_phi =
                                self.builder.build_phi(i64_ty, "io_i").unwrap();
                            i_phi.add_incoming(&[(&zero_i64, io_pre_bb)]);
                            let i_val = i_phi.as_basic_value().into_int_value();

                            let limit = self
                                .builder
                                .build_int_sub(hay_len, ndl_len, "io_limit")
                                .unwrap();
                            let limit_p1 = self
                                .builder
                                .build_int_add(limit, one_i64, "io_limit_p1")
                                .unwrap();
                            let in_range = self
                                .builder
                                .build_int_compare(
                                    IntPredicate::ULT, i_val, limit_p1, "io_in_range",
                                )
                                .unwrap();
                            self.builder
                                .build_conditional_branch(in_range, io_check_bb, io_not_bb)
                                .unwrap();

                            self.builder.position_at_end(io_check_bb);
                            let sub_ptr = unsafe {
                                self.builder
                                    .build_in_bounds_gep(i8_ty, hay_ptr, &[i_val], "io_sub")
                                    .unwrap()
                            };
                            let cmp = self
                                .builder
                                .build_call(
                                    memcmp_fn,
                                    &[sub_ptr.into(), ndl_ptr.into(), ndl_len.into()],
                                    "io_cmp",
                                )
                                .unwrap()
                                .try_as_basic_value()
                                .basic()
                                .unwrap()
                                .into_int_value();
                            let found = self
                                .builder
                                .build_int_compare(
                                    IntPredicate::EQ, cmp, zero_i32, "io_found",
                                )
                                .unwrap();
                            self.builder
                                .build_conditional_branch(found, io_found_bb, io_inc_bb)
                                .unwrap();

                            self.builder.position_at_end(io_inc_bb);
                            let next_i = self
                                .builder
                                .build_int_add(i_val, one_i64, "io_next_i")
                                .unwrap();
                            i_phi.add_incoming(&[(&next_i, io_inc_bb)]);
                            self.builder
                                .build_unconditional_branch(io_loop_bb)
                                .unwrap();

                            // Found: return i as i32
                            self.builder.position_at_end(io_found_bb);
                            let found_idx = self
                                .builder
                                .build_int_truncate(i_val, i32_ty, "io_idx_i32")
                                .unwrap();
                            self.builder
                                .build_unconditional_branch(io_merge_bb)
                                .unwrap();

                            // Not found: return -1
                            self.builder.position_at_end(io_not_bb);
                            self.builder
                                .build_unconditional_branch(io_merge_bb)
                                .unwrap();

                            self.builder.position_at_end(io_merge_bb);
                            let phi = self
                                .builder
                                .build_phi(i32_ty, "io_result")
                                .unwrap();
                            phi.add_incoming(&[
                                (&found_idx, io_found_bb),
                                (&neg1_i32, io_not_bb),
                            ]);
                            return Ok(Some(phi.as_basic_value()));
                        }
                        "contains" | "starts_with" | "ends_with" => {
                            // String comparison methods via memcmp-based loop
                            let needle_val = self
                                .compile_expr(&args[0], function)?
                                .unwrap()
                                .into_struct_value();

                            let hay_ptr = self
                                .builder
                                .build_extract_value(str_val, 0, "hay_ptr")
                                .unwrap()
                                .into_pointer_value();
                            let hay_len = self
                                .builder
                                .build_extract_value(str_val, 1, "hay_len")
                                .unwrap()
                                .into_int_value();
                            let ndl_ptr = self
                                .builder
                                .build_extract_value(needle_val, 0, "ndl_ptr")
                                .unwrap()
                                .into_pointer_value();
                            let ndl_len = self
                                .builder
                                .build_extract_value(needle_val, 1, "ndl_len")
                                .unwrap()
                                .into_int_value();

                            let memcmp_fn = self.get_or_declare_memcmp();
                            let i8_ty = self.context.i8_type();
                            let i64_ty = self.context.i64_type();
                            let zero_i64 = i64_ty.const_int(0, false);
                            let one_i64 = i64_ty.const_int(1, false);
                            let zero_i32 = self.context.i32_type().const_int(0, false);

                            match method.as_str() {
                                "starts_with" => {
                                    // ndl_len <= hay_len && memcmp(hay_ptr, ndl_ptr, ndl_len) == 0
                                    let len_ok = self
                                        .builder
                                        .build_int_compare(
                                            IntPredicate::ULE,
                                            ndl_len,
                                            hay_len,
                                            "sw_len_ok",
                                        )
                                        .unwrap();

                                    let sw_check_bb =
                                        self.context.append_basic_block(*function, "sw_check");
                                    let sw_false_bb =
                                        self.context.append_basic_block(*function, "sw_false");
                                    let sw_merge_bb =
                                        self.context.append_basic_block(*function, "sw_merge");

                                    self.builder
                                        .build_conditional_branch(len_ok, sw_check_bb, sw_false_bb)
                                        .unwrap();

                                    self.builder.position_at_end(sw_check_bb);
                                    let cmp = self
                                        .builder
                                        .build_call(
                                            memcmp_fn,
                                            &[hay_ptr.into(), ndl_ptr.into(), ndl_len.into()],
                                            "sw_cmp",
                                        )
                                        .unwrap()
                                        .try_as_basic_value()
                                        .basic()
                                        .unwrap()
                                        .into_int_value();
                                    let is_eq = self
                                        .builder
                                        .build_int_compare(IntPredicate::EQ, cmp, zero_i32, "sw_eq")
                                        .unwrap();
                                    self.builder
                                        .build_unconditional_branch(sw_merge_bb)
                                        .unwrap();

                                    self.builder.position_at_end(sw_false_bb);
                                    let false_val =
                                        self.context.bool_type().const_int(0, false);
                                    self.builder
                                        .build_unconditional_branch(sw_merge_bb)
                                        .unwrap();

                                    self.builder.position_at_end(sw_merge_bb);
                                    let phi = self
                                        .builder
                                        .build_phi(self.context.bool_type(), "sw_result")
                                        .unwrap();
                                    phi.add_incoming(&[
                                        (&is_eq, sw_check_bb),
                                        (&false_val, sw_false_bb),
                                    ]);
                                    return Ok(Some(phi.as_basic_value()));
                                }
                                "ends_with" => {
                                    // ndl_len <= hay_len && memcmp(hay_ptr + (hay_len - ndl_len), ndl_ptr, ndl_len) == 0
                                    let len_ok = self
                                        .builder
                                        .build_int_compare(
                                            IntPredicate::ULE,
                                            ndl_len,
                                            hay_len,
                                            "ew_len_ok",
                                        )
                                        .unwrap();

                                    let ew_check_bb =
                                        self.context.append_basic_block(*function, "ew_check");
                                    let ew_false_bb =
                                        self.context.append_basic_block(*function, "ew_false");
                                    let ew_merge_bb =
                                        self.context.append_basic_block(*function, "ew_merge");

                                    self.builder
                                        .build_conditional_branch(len_ok, ew_check_bb, ew_false_bb)
                                        .unwrap();

                                    self.builder.position_at_end(ew_check_bb);
                                    let offset = self
                                        .builder
                                        .build_int_sub(hay_len, ndl_len, "ew_offset")
                                        .unwrap();
                                    let tail_ptr = unsafe {
                                        self.builder
                                            .build_in_bounds_gep(
                                                i8_ty, hay_ptr, &[offset], "ew_tail",
                                            )
                                            .unwrap()
                                    };
                                    let cmp = self
                                        .builder
                                        .build_call(
                                            memcmp_fn,
                                            &[tail_ptr.into(), ndl_ptr.into(), ndl_len.into()],
                                            "ew_cmp",
                                        )
                                        .unwrap()
                                        .try_as_basic_value()
                                        .basic()
                                        .unwrap()
                                        .into_int_value();
                                    let is_eq = self
                                        .builder
                                        .build_int_compare(IntPredicate::EQ, cmp, zero_i32, "ew_eq")
                                        .unwrap();
                                    self.builder
                                        .build_unconditional_branch(ew_merge_bb)
                                        .unwrap();

                                    self.builder.position_at_end(ew_false_bb);
                                    let false_val =
                                        self.context.bool_type().const_int(0, false);
                                    self.builder
                                        .build_unconditional_branch(ew_merge_bb)
                                        .unwrap();

                                    self.builder.position_at_end(ew_merge_bb);
                                    let phi = self
                                        .builder
                                        .build_phi(self.context.bool_type(), "ew_result")
                                        .unwrap();
                                    phi.add_incoming(&[
                                        (&is_eq, ew_check_bb),
                                        (&false_val, ew_false_bb),
                                    ]);
                                    return Ok(Some(phi.as_basic_value()));
                                }
                                "contains" => {
                                    // Naive O(n*m) substring search
                                    let ct_pre_bb = self.builder.get_insert_block().unwrap();
                                    let ct_loop_bb =
                                        self.context.append_basic_block(*function, "ct_loop");
                                    let ct_check_bb =
                                        self.context.append_basic_block(*function, "ct_check");
                                    let ct_inc_bb =
                                        self.context.append_basic_block(*function, "ct_inc");
                                    let ct_found_bb =
                                        self.context.append_basic_block(*function, "ct_found");
                                    let ct_not_bb =
                                        self.context.append_basic_block(*function, "ct_not");
                                    let ct_merge_bb =
                                        self.context.append_basic_block(*function, "ct_merge");

                                    // If ndl_len > hay_len, jump to not found
                                    let len_ok = self
                                        .builder
                                        .build_int_compare(
                                            IntPredicate::ULE,
                                            ndl_len,
                                            hay_len,
                                            "ct_len_ok",
                                        )
                                        .unwrap();
                                    self.builder
                                        .build_conditional_branch(len_ok, ct_loop_bb, ct_not_bb)
                                        .unwrap();

                                    // Loop header: i = 0 .. hay_len - ndl_len
                                    self.builder.position_at_end(ct_loop_bb);
                                    let i_phi = self
                                        .builder
                                        .build_phi(i64_ty, "ct_i")
                                        .unwrap();
                                    i_phi.add_incoming(&[(&zero_i64, ct_pre_bb)]);
                                    let i_val = i_phi.as_basic_value().into_int_value();

                                    let limit = self
                                        .builder
                                        .build_int_sub(hay_len, ndl_len, "ct_limit")
                                        .unwrap();
                                    let limit_plus = self
                                        .builder
                                        .build_int_add(limit, one_i64, "ct_limit_p1")
                                        .unwrap();
                                    let in_range = self
                                        .builder
                                        .build_int_compare(
                                            IntPredicate::ULT,
                                            i_val,
                                            limit_plus,
                                            "ct_in_range",
                                        )
                                        .unwrap();
                                    self.builder
                                        .build_conditional_branch(in_range, ct_check_bb, ct_not_bb)
                                        .unwrap();

                                    // Check: memcmp(hay_ptr + i, ndl_ptr, ndl_len) == 0
                                    self.builder.position_at_end(ct_check_bb);
                                    let sub_ptr = unsafe {
                                        self.builder
                                            .build_in_bounds_gep(
                                                i8_ty, hay_ptr, &[i_val], "ct_sub",
                                            )
                                            .unwrap()
                                    };
                                    let cmp = self
                                        .builder
                                        .build_call(
                                            memcmp_fn,
                                            &[sub_ptr.into(), ndl_ptr.into(), ndl_len.into()],
                                            "ct_cmp",
                                        )
                                        .unwrap()
                                        .try_as_basic_value()
                                        .basic()
                                        .unwrap()
                                        .into_int_value();
                                    let found = self
                                        .builder
                                        .build_int_compare(
                                            IntPredicate::EQ, cmp, zero_i32, "ct_found",
                                        )
                                        .unwrap();
                                    self.builder
                                        .build_conditional_branch(found, ct_found_bb, ct_inc_bb)
                                        .unwrap();

                                    // Increment block: i += 1, then back to loop
                                    self.builder.position_at_end(ct_inc_bb);
                                    let next_i = self
                                        .builder
                                        .build_int_add(i_val, one_i64, "ct_next_i")
                                        .unwrap();
                                    i_phi.add_incoming(&[(&next_i, ct_inc_bb)]);
                                    self.builder
                                        .build_unconditional_branch(ct_loop_bb)
                                        .unwrap();

                                    // Found
                                    self.builder.position_at_end(ct_found_bb);
                                    let true_val = self.context.bool_type().const_int(1, false);
                                    self.builder
                                        .build_unconditional_branch(ct_merge_bb)
                                        .unwrap();

                                    // Not found
                                    self.builder.position_at_end(ct_not_bb);
                                    let false_val =
                                        self.context.bool_type().const_int(0, false);
                                    self.builder
                                        .build_unconditional_branch(ct_merge_bb)
                                        .unwrap();

                                    // Merge
                                    self.builder.position_at_end(ct_merge_bb);
                                    let phi = self
                                        .builder
                                        .build_phi(self.context.bool_type(), "ct_result")
                                        .unwrap();
                                    phi.add_incoming(&[
                                        (&true_val, ct_found_bb),
                                        (&false_val, ct_not_bb),
                                    ]);
                                    return Ok(Some(phi.as_basic_value()));
                                }
                                _ => unreachable!(),
                            }
                        }
                        _ => {}
                    }
                }

                // Compile the object as the first argument (pass by value or pointer)
                let obj_val = self.compile_expr(object, function)?.unwrap();
                let callee = method.clone();

                if let Some((func, _, ret_ty)) = self.functions.get(&callee).cloned() {
                    let mut arg_values = vec![obj_val.into()];
                    for arg in args {
                        let val = self.compile_expr(arg, function)?.unwrap();
                        arg_values.push(val.into());
                    }
                    let call = self
                        .builder
                        .build_call(func, &arg_values, "method_call")
                        .unwrap();
                    if ret_ty == NyType::Unit {
                        Ok(None)
                    } else {
                        Ok(call.try_as_basic_value().basic())
                    }
                } else {
                    // Try struct_name.method_name convention
                    let struct_name = match &obj_ty {
                        NyType::Struct { name, .. } => name.clone(),
                        NyType::Pointer(inner) => match inner.as_ref() {
                            NyType::Struct { name, .. } => name.clone(),
                            _ => String::new(),
                        },
                        _ => String::new(),
                    };
                    let qualified_name = format!("{}_{}", struct_name, method);
                    if let Some((func, _, ret_ty)) = self.functions.get(&qualified_name).cloned() {
                        let mut arg_values = vec![obj_val.into()];
                        for arg in args {
                            let val = self.compile_expr(arg, function)?.unwrap();
                            arg_values.push(val.into());
                        }
                        let call = self
                            .builder
                            .build_call(func, &arg_values, "method_call")
                            .unwrap();
                        if ret_ty == NyType::Unit {
                            Ok(None)
                        } else {
                            Ok(call.try_as_basic_value().basic())
                        }
                    } else {
                        Err(vec![CompileError::name_error(
                            format!("undeclared method '{}'", method),
                            expr.span(),
                        )])
                    }
                }
            }

            // ---- Type cast (expr as T) ----
            Expr::Cast {
                expr: inner_expr,
                target_type,
                ..
            } => {
                let val = self.compile_expr(inner_expr, function)?.unwrap();
                let source_ty = self.infer_expr_type(inner_expr);
                let target_ty = self.resolve_type_annotation(target_type);
                let target_llvm = ny_to_llvm(self.context, &target_ty);

                if source_ty == target_ty {
                    return Ok(Some(val)); // no-op
                }

                let result = self.compile_cast(val, &source_ty, &target_ty, target_llvm)?;
                Ok(Some(result))
            }

            // ---- Enum variant ----
            // ---- Try (?) operator ----
            Expr::Try { operand, .. } => {
                let subject_raw = self.compile_expr(operand, function)?.unwrap();
                let subject_ty = self.infer_expr_type(operand);

                let enum_name = match &subject_ty {
                    NyType::Enum { name, .. } => name.clone(),
                    _ => {
                        return Err(vec![CompileError::type_error(
                            "? requires enum type".to_string(),
                            expr.span(),
                        )]);
                    }
                };

                if self.enum_has_payload(&enum_name) {
                    let enum_ty = self.enum_struct_type(&enum_name);
                    let alloca = self.builder.build_alloca(enum_ty, "try_subject").unwrap();
                    self.builder.build_store(alloca, subject_raw).unwrap();

                    // Extract tag
                    let tag_ptr = self
                        .builder
                        .build_struct_gep(enum_ty, alloca, 0, "try_tag_ptr")
                        .unwrap();
                    let tag = self
                        .builder
                        .build_load(self.context.i32_type(), tag_ptr, "try_tag")
                        .unwrap()
                        .into_int_value();

                    let ok_bb = self.context.append_basic_block(*function, "try_ok");
                    let err_bb = self.context.append_basic_block(*function, "try_err");

                    // tag == 0 means first variant (Ok)
                    let zero = self.context.i32_type().const_zero();
                    let is_ok = self
                        .builder
                        .build_int_compare(IntPredicate::EQ, tag, zero, "is_ok")
                        .unwrap();
                    self.builder
                        .build_conditional_branch(is_ok, ok_bb, err_bb)
                        .unwrap();

                    // Err path: return the whole enum from the current function
                    self.builder.position_at_end(err_bb);
                    // Emit defers before early return
                    let defers: Vec<(Expr, FunctionValue<'ctx>)> =
                        self.defer_stack.iter().rev().cloned().collect();
                    for (defer_body, defer_fn) in &defers {
                        self.compile_expr(defer_body, defer_fn)?;
                    }
                    self.builder.build_return(Some(&subject_raw)).unwrap();

                    // Ok path: extract the first payload field
                    self.builder.position_at_end(ok_bb);
                    let payload_ptr = self
                        .builder
                        .build_struct_gep(enum_ty, alloca, 1, "try_payload_ptr")
                        .unwrap();
                    let result_ty = self.infer_expr_type(expr);
                    let payload_llvm = ny_to_llvm(self.context, &result_ty);
                    let payload_val = self
                        .builder
                        .build_load(payload_llvm, payload_ptr, "try_payload")
                        .unwrap();
                    Ok(Some(payload_val))
                } else {
                    // Simple enum — can't use ? on it meaningfully
                    Ok(Some(subject_raw))
                }
            }

            // ---- Lambda (with optional captures via lambda lifting) ----
            Expr::Lambda {
                params,
                return_type,
                body,
                ..
            } => {
                static LAMBDA_COUNTER: std::sync::atomic::AtomicUsize =
                    std::sync::atomic::AtomicUsize::new(0);
                let id = LAMBDA_COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                let lambda_name = format!("__lambda_{}", id);

                let ret_ty = self.resolve_type_annotation(return_type);
                let param_types: Vec<NyType> = params
                    .iter()
                    .map(|p| self.resolve_type_annotation(&p.ty))
                    .collect();
                let param_names: Vec<String> = params.iter().map(|p| p.name.clone()).collect();

                // Find captured variables: scan body for Ident references
                // that are not in the lambda's param list but exist in outer scope
                let mut captures: Vec<(String, NyType)> = Vec::new();
                let mut capture_values: Vec<BasicValueEnum<'ctx>> = Vec::new();
                {
                    let free_vars = find_free_vars(body, &param_names);
                    for var_name in &free_vars {
                        if let Some((ptr, ty)) = self.variables.get(var_name) {
                            let llvm_ty = ny_to_llvm(self.context, ty);
                            let val = self
                                .builder
                                .build_load(llvm_ty, *ptr, &format!("cap_{}", var_name))
                                .unwrap();
                            captures.push((var_name.clone(), ty.clone()));
                            capture_values.push(val);
                        }
                    }
                }

                // Build the function with captures as prefix params
                let mut all_param_types: Vec<NyType> = Vec::new();
                for (_, ty) in &captures {
                    all_param_types.push(ty.clone());
                }
                all_param_types.extend(param_types.clone());

                let llvm_param_types: Vec<BasicTypeEnum> = all_param_types
                    .iter()
                    .map(|t| ny_to_llvm(self.context, t))
                    .collect();
                let param_meta: Vec<_> = llvm_param_types.iter().map(|t| (*t).into()).collect();

                let fn_type = match &ret_ty {
                    NyType::Unit => self.context.void_type().fn_type(&param_meta, false),
                    ty => ny_to_llvm(self.context, ty).fn_type(&param_meta, false),
                };

                let lambda_fn = self.module.add_function(&lambda_name, fn_type, None);
                self.functions.insert(
                    lambda_name.clone(),
                    (lambda_fn, all_param_types.clone(), ret_ty),
                );

                // Save state (variables + defer stack)
                let outer_vars = self.variables.clone();
                let outer_defers = std::mem::take(&mut self.defer_stack);
                self.variables.clear();
                let current_bb = self.builder.get_insert_block().unwrap();

                // Build lambda body
                let entry = self.context.append_basic_block(lambda_fn, "entry");
                self.builder.position_at_end(entry);

                // Set up captured variables as params
                for (i, (name, ty)) in captures.iter().enumerate() {
                    let llvm_ty = ny_to_llvm(self.context, ty);
                    let alloca = self.builder.build_alloca(llvm_ty, name).unwrap();
                    self.builder
                        .build_store(alloca, lambda_fn.get_nth_param(i as u32).unwrap())
                        .unwrap();
                    self.variables.insert(name.clone(), (alloca, ty.clone()));
                }

                // Set up explicit params
                let cap_count = captures.len();
                for (i, param) in params.iter().enumerate() {
                    let ty = &param_types[i];
                    let llvm_ty = ny_to_llvm(self.context, ty);
                    let alloca = self.builder.build_alloca(llvm_ty, &param.name).unwrap();
                    self.builder
                        .build_store(
                            alloca,
                            lambda_fn.get_nth_param((cap_count + i) as u32).unwrap(),
                        )
                        .unwrap();
                    self.variables
                        .insert(param.name.clone(), (alloca, ty.clone()));
                }

                self.compile_expr(body, &lambda_fn)?;

                let lambda_end_bb = self.builder.get_insert_block().unwrap();
                if lambda_end_bb.get_terminator().is_none() {
                    self.builder.build_return(None).unwrap();
                }

                // Restore state (variables + defer stack)
                self.variables = outer_vars;
                self.defer_stack = outer_defers;
                self.builder.position_at_end(current_bb);

                let fn_ptr = lambda_fn.as_global_value().as_pointer_value();

                if !captures.is_empty() {
                    // Create dedicated allocas to store captured values by-value
                    // These persist after the lambda is created, even if the original
                    // variable is reassigned.
                    let mut capture_alloca_names: Vec<(String, NyType)> = Vec::new();
                    for ((cap_name, cap_ty), cap_val) in captures.iter().zip(capture_values.iter())
                    {
                        let alloca_name = format!("__cl{}_{}", id, cap_name);
                        let llvm_ty = ny_to_llvm(self.context, cap_ty);
                        let alloca = self.builder.build_alloca(llvm_ty, &alloca_name).unwrap();
                        self.builder.build_store(alloca, *cap_val).unwrap();
                        self.variables
                            .insert(alloca_name.clone(), (alloca, cap_ty.clone()));
                        capture_alloca_names.push((alloca_name, cap_ty.clone()));
                    }

                    self.closure_captures
                        .insert(lambda_name.clone(), (lambda_name, capture_alloca_names));
                }

                Ok(Some(fn_ptr.into()))
            }

            // ---- Range index (arr[start..end] → slice {ptr, len}) ----
            Expr::RangeIndex {
                object, start, end, ..
            } => {
                let obj_ty = self.infer_expr_type(object);
                let start_val = self
                    .compile_expr(start, function)?
                    .unwrap()
                    .into_int_value();
                let end_val = self.compile_expr(end, function)?.unwrap().into_int_value();

                let start_i64 = self
                    .builder
                    .build_int_z_extend_or_bit_cast(start_val, self.context.i64_type(), "start_ext")
                    .unwrap();
                let end_i64 = self
                    .builder
                    .build_int_z_extend_or_bit_cast(end_val, self.context.i64_type(), "end_ext")
                    .unwrap();

                match &obj_ty {
                    NyType::Array { elem, size } => {
                        let elem_llvm = ny_to_llvm(self.context, elem);
                        let arr_llvm = elem_llvm.array_type(*size as u32);
                        let arr_ptr = self.compile_expr_as_ptr(object, function)?;
                        let zero = self.context.i64_type().const_zero();
                        let gep = unsafe {
                            self.builder
                                .build_in_bounds_gep(
                                    arr_llvm,
                                    arr_ptr,
                                    &[zero, start_i64],
                                    "slice_ptr",
                                )
                                .unwrap()
                        };
                        let len = self
                            .builder
                            .build_int_sub(end_i64, start_i64, "slice_len")
                            .unwrap();
                        // Build {ptr, len} slice struct
                        let slice_ty = self.context.struct_type(
                            &[
                                self.context.ptr_type(AddressSpace::default()).into(),
                                self.context.i64_type().into(),
                            ],
                            false,
                        );
                        let slice_val = slice_ty.const_zero();
                        let slice_val = self
                            .builder
                            .build_insert_value(slice_val, gep, 0, "slice_p")
                            .unwrap();
                        let slice_val = self
                            .builder
                            .build_insert_value(slice_val, len, 1, "slice_l")
                            .unwrap();
                        Ok(Some(slice_val.into_struct_value().into()))
                    }
                    _ => Err(vec![CompileError::type_error(
                        "range index on non-array type".to_string(),
                        object.span(),
                    )]),
                }
            }

            Expr::EnumVariant {
                enum_name,
                variant,
                args,
                ..
            } => {
                if let Some(variant_defs) = self.enum_variants.get(enum_name).cloned() {
                    let idx = variant_defs
                        .iter()
                        .position(|(name, _)| name == variant)
                        .unwrap_or(0);
                    let tag_val = self.context.i32_type().const_int(idx as u64, false);

                    if !self.enum_has_payload(enum_name) || args.is_empty() {
                        // Simple enum — just return the discriminant
                        Ok(Some(tag_val.into()))
                    } else {
                        // Data-carrying enum — build { tag, payload... } struct
                        let enum_ty = self.enum_struct_type(enum_name);
                        let alloca = self.builder.build_alloca(enum_ty, "enum_val").unwrap();

                        // Store tag
                        let tag_ptr = self
                            .builder
                            .build_struct_gep(enum_ty, alloca, 0, "tag_ptr")
                            .unwrap();
                        self.builder.build_store(tag_ptr, tag_val).unwrap();

                        // Store payload fields
                        for (i, arg) in args.iter().enumerate() {
                            let val = self.compile_expr(arg, function)?.unwrap();
                            let field_ptr = self
                                .builder
                                .build_struct_gep(
                                    enum_ty,
                                    alloca,
                                    (i + 1) as u32,
                                    &format!("payload_{}", i),
                                )
                                .unwrap();
                            self.builder.build_store(field_ptr, val).unwrap();
                        }

                        let result = self
                            .builder
                            .build_load(enum_ty, alloca, "enum_loaded")
                            .unwrap();
                        Ok(Some(result))
                    }
                } else {
                    Err(vec![CompileError::type_error(
                        format!("unknown enum '{}'", enum_name),
                        expr.span(),
                    )])
                }
            }

            // ---- Match expression ----
            Expr::Match { subject, arms, .. } => {
                let subject_raw = self.compile_expr(subject, function)?.unwrap();
                let subject_ty = self.infer_expr_type(subject);
                let result_ty = self.infer_expr_type(expr);
                let has_result = result_ty != NyType::Unit;

                // Determine if subject is a data-carrying enum
                let is_tagged_union = match &subject_ty {
                    NyType::Enum { name, .. } => self.enum_has_payload(name),
                    _ => false,
                };

                // Extract the tag (discriminant) for the switch
                let (subject_val, subject_ptr) = if is_tagged_union {
                    let enum_name = match &subject_ty {
                        NyType::Enum { name, .. } => name.clone(),
                        _ => unreachable!(),
                    };
                    let enum_ty = self.enum_struct_type(&enum_name);
                    // Store subject to alloca so we can GEP into it
                    let alloca = self.builder.build_alloca(enum_ty, "match_subject").unwrap();
                    self.builder.build_store(alloca, subject_raw).unwrap();
                    // Extract tag
                    let tag_ptr = self
                        .builder
                        .build_struct_gep(enum_ty, alloca, 0, "tag_ptr")
                        .unwrap();
                    let tag = self
                        .builder
                        .build_load(self.context.i32_type(), tag_ptr, "tag")
                        .unwrap()
                        .into_int_value();
                    (tag, Some(alloca))
                } else {
                    (subject_raw.into_int_value(), None)
                };

                let merge_bb = self.context.append_basic_block(*function, "match_merge");

                let wildcard_idx = arms
                    .iter()
                    .position(|arm| matches!(arm.pattern, Pattern::Wildcard(_)));

                let arm_bbs: Vec<BasicBlock> = arms
                    .iter()
                    .enumerate()
                    .map(|(i, _)| {
                        self.context
                            .append_basic_block(*function, &format!("match_arm_{}", i))
                    })
                    .collect();

                let switch_bb = self.builder.get_insert_block().unwrap();

                let default_bb = if let Some(wi) = wildcard_idx {
                    arm_bbs[wi]
                } else {
                    let unreachable_bb = self
                        .context
                        .append_basic_block(*function, "match_unreachable");
                    self.builder.position_at_end(unreachable_bb);
                    self.builder.build_unreachable().unwrap();
                    unreachable_bb
                };

                let mut cases: Vec<(inkwell::values::IntValue<'ctx>, BasicBlock<'ctx>)> =
                    Vec::new();
                for (i, arm) in arms.iter().enumerate() {
                    match &arm.pattern {
                        Pattern::EnumVariant {
                            enum_name, variant, ..
                        } => {
                            if let Some(variant_defs) = self.enum_variants.get(enum_name) {
                                let idx = variant_defs
                                    .iter()
                                    .position(|(name, _)| name == variant)
                                    .unwrap_or(0);
                                let const_val =
                                    self.context.i32_type().const_int(idx as u64, false);
                                cases.push((const_val, arm_bbs[i]));
                            }
                        }
                        Pattern::IntLit(n, _) => {
                            let const_val = self.context.i32_type().const_int(*n as u64, true);
                            cases.push((const_val, arm_bbs[i]));
                        }
                        Pattern::Wildcard(_) => {}
                    }
                }

                self.builder.position_at_end(switch_bb);
                self.builder
                    .build_switch(subject_val, default_bb, &cases)
                    .unwrap();

                // Compile each arm body, extracting payload bindings for tagged unions
                let mut incoming: Vec<(BasicValueEnum<'ctx>, BasicBlock<'ctx>)> = Vec::new();
                for (i, arm) in arms.iter().enumerate() {
                    self.builder.position_at_end(arm_bbs[i]);

                    // Extract payload bindings for data-carrying enum patterns
                    if let Pattern::EnumVariant {
                        enum_name,
                        bindings,
                        variant,
                        ..
                    } = &arm.pattern
                    {
                        if !bindings.is_empty() {
                            if let Some(alloca) = subject_ptr {
                                let enum_ty = self.enum_struct_type(enum_name);
                                // Find the variant's payload types
                                let payload_types = self
                                    .enum_variants
                                    .get(enum_name)
                                    .and_then(|vs| {
                                        vs.iter()
                                            .find(|(n, _)| n == variant)
                                            .map(|(_, p)| p.clone())
                                    })
                                    .unwrap_or_default();

                                for (j, binding_name) in bindings.iter().enumerate() {
                                    let field_idx = (j + 1) as u32; // skip tag
                                    let field_ptr = self
                                        .builder
                                        .build_struct_gep(
                                            enum_ty,
                                            alloca,
                                            field_idx,
                                            &format!("bind_{}", binding_name),
                                        )
                                        .unwrap();
                                    let payload_ny_ty =
                                        payload_types.get(j).cloned().unwrap_or(NyType::I32);
                                    let payload_llvm_ty = ny_to_llvm(self.context, &payload_ny_ty);
                                    let val = self
                                        .builder
                                        .build_load(payload_llvm_ty, field_ptr, binding_name)
                                        .unwrap();
                                    // Declare binding as a variable
                                    let bind_alloca = self
                                        .builder
                                        .build_alloca(payload_llvm_ty, binding_name)
                                        .unwrap();
                                    self.builder.build_store(bind_alloca, val).unwrap();
                                    self.variables
                                        .insert(binding_name.clone(), (bind_alloca, payload_ny_ty));
                                }
                            }
                        }
                    }

                    let arm_val = self.compile_expr(&arm.body, function)?;
                    let arm_end_bb = self.builder.get_insert_block().unwrap();
                    let has_terminator = arm_end_bb.get_terminator().is_some();
                    if !has_terminator {
                        if has_result {
                            if let Some(v) = arm_val {
                                incoming.push((v, arm_end_bb));
                            }
                        }
                        self.builder.build_unconditional_branch(merge_bb).unwrap();
                    }
                }

                self.builder.position_at_end(merge_bb);

                if has_result && !incoming.is_empty() {
                    let llvm_ty = ny_to_llvm(self.context, &result_ty);
                    let phi = self.builder.build_phi(llvm_ty, "match_val").unwrap();
                    let refs: Vec<(&dyn inkwell::values::BasicValue, BasicBlock)> = incoming
                        .iter()
                        .map(|(v, bb)| (v as &dyn inkwell::values::BasicValue, *bb))
                        .collect();
                    phi.add_incoming(&refs);
                    Ok(Some(phi.as_basic_value()))
                } else {
                    Ok(None)
                }
            }

            // ---- Tuple literal ----
            Expr::TupleLit { elements, .. } => {
                let mut elem_values = Vec::new();
                for elem in elements {
                    let val = self.compile_expr(elem, function)?.unwrap();
                    elem_values.push(val);
                }

                let elem_types: Vec<BasicTypeEnum> =
                    elem_values.iter().map(|v| v.get_type()).collect();
                let tuple_ty = self.context.struct_type(&elem_types, false);
                let alloca = self.builder.build_alloca(tuple_ty, "tuple").unwrap();

                for (i, val) in elem_values.iter().enumerate() {
                    let field_ptr = self
                        .builder
                        .build_struct_gep(tuple_ty, alloca, i as u32, &format!("tuple_field_{}", i))
                        .unwrap();
                    self.builder.build_store(field_ptr, *val).unwrap();
                }

                let tuple_val = self
                    .builder
                    .build_load(tuple_ty, alloca, "tuple_val")
                    .unwrap();
                Ok(Some(tuple_val))
            }

            // ---- Tuple index ----
            Expr::TupleIndex { object, index, .. } => {
                let obj_val = self.compile_expr(object, function)?.unwrap();
                let struct_val = obj_val.into_struct_value();
                let extracted = self
                    .builder
                    .build_extract_value(struct_val, *index as u32, "tuple_idx")
                    .unwrap();
                Ok(Some(extracted))
            }

            // ---- Await ----
            Expr::Await { future, .. } => {
                let future_ptr = self.compile_expr(future, function)?.unwrap();
                let await_fn = self.get_or_declare_ny_future_await();
                let result = self
                    .builder
                    .build_call(await_fn, &[future_ptr.into()], "await_val")
                    .unwrap()
                    .try_as_basic_value()
                    .basic()
                    .unwrap();

                // The future returns i64. Cast to the expected type.
                let inner_ty = match self.infer_expr_type(future) {
                    NyType::Future(inner) => *inner,
                    _ => NyType::I32,
                };
                if inner_ty == NyType::I32 {
                    let truncated = self
                        .builder
                        .build_int_truncate(
                            result.into_int_value(),
                            self.context.i32_type(),
                            "await_i32",
                        )
                        .unwrap();
                    Ok(Some(truncated.into()))
                } else {
                    Ok(Some(result))
                }
            }
        }
    }

    // ------------------------------------------------------------------
    // Compile an expression and return a pointer to its storage.
    // Used for &expr, array indexing (need alloca ptr), field access, etc.
    // ------------------------------------------------------------------

    pub(super) fn compile_expr_as_ptr(
        &mut self,
        expr: &Expr,
        function: &FunctionValue<'ctx>,
    ) -> Result<PointerValue<'ctx>, Vec<CompileError>> {
        match expr {
            Expr::Ident { name, .. } => {
                if let Some((ptr, _)) = self.variables.get(name) {
                    Ok(*ptr)
                } else {
                    Err(vec![CompileError::name_error(
                        format!("undeclared variable '{}'", name),
                        expr.span(),
                    )])
                }
            }
            Expr::Index { object, index, .. } => {
                // Return the GEP pointer to the element
                let obj_ty = self.infer_expr_type(object);
                let idx_val = self
                    .compile_expr(index, function)?
                    .unwrap()
                    .into_int_value();

                match &obj_ty {
                    NyType::Array { elem, size } => {
                        let elem_llvm = ny_to_llvm(self.context, elem);
                        let arr_llvm = elem_llvm.array_type(*size as u32);
                        let arr_ptr = self.compile_expr_as_ptr(object, function)?;

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
                                .build_in_bounds_gep(arr_llvm, arr_ptr, &[zero, idx_i64], "idx_ptr")
                                .unwrap()
                        };
                        Ok(gep)
                    }
                    _ => Err(vec![CompileError::type_error(
                        "cannot index non-array type".to_string(),
                        object.span(),
                    )]),
                }
            }
            Expr::FieldAccess { object, field, .. } => {
                let obj_ty = self.infer_expr_type(object);

                let (struct_name, struct_fields, is_pointer) = match &obj_ty {
                    NyType::Struct { name, fields } => (name.clone(), fields.clone(), false),
                    NyType::Pointer(inner) => match inner.as_ref() {
                        NyType::Struct { name, fields } => (name.clone(), fields.clone(), true),
                        _ => {
                            return Err(vec![CompileError::type_error(
                                "field access on non-struct pointer".to_string(),
                                object.span(),
                            )]);
                        }
                    },
                    _ => {
                        return Err(vec![CompileError::type_error(
                            format!("field access on non-struct type '{}'", obj_ty),
                            object.span(),
                        )]);
                    }
                };

                let field_idx = struct_fields
                    .iter()
                    .position(|(n, _)| n == field)
                    .unwrap_or(0) as u32;
                let struct_llvm_ty =
                    self.get_or_create_llvm_struct_type(&struct_name, &struct_fields);

                let struct_ptr = if is_pointer {
                    let ptr_val = self.compile_expr(object, function)?.unwrap();
                    ptr_val.into_pointer_value()
                } else {
                    self.compile_expr_as_ptr(object, function)?
                };

                let field_ptr = self
                    .builder
                    .build_struct_gep(
                        struct_llvm_ty,
                        struct_ptr,
                        field_idx,
                        &format!("{}_field_ptr", field),
                    )
                    .unwrap();
                Ok(field_ptr)
            }
            Expr::Deref { operand, .. } => {
                // *ptr — the pointer itself is the address we want
                let ptr_val = self.compile_expr(operand, function)?.unwrap();
                Ok(ptr_val.into_pointer_value())
            }
            _ => {
                // For arbitrary expressions, evaluate and store in a temporary alloca
                let val = self.compile_expr(expr, function)?.unwrap();
                let alloca = self.builder.build_alloca(val.get_type(), "tmp").unwrap();
                self.builder.build_store(alloca, val).unwrap();
                Ok(alloca)
            }
        }
    }
}
