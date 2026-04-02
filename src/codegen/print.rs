use inkwell::basic_block::BasicBlock;
use inkwell::values::{BasicValueEnum, FunctionValue};
use inkwell::{AddressSpace, IntPredicate};

use crate::common::{CompileError, NyType};
use crate::parser::ast::Expr;

use super::types::{ny_to_llvm, str_type};
use super::CodeGen;

impl<'ctx> CodeGen<'ctx> {
    // ------------------------------------------------------------------
    // String literal: build a global constant and return {ptr, len} struct
    // ------------------------------------------------------------------

    pub(super) fn build_str_literal(&self, s: &str) -> BasicValueEnum<'ctx> {
        let str_ty = str_type(self.context);
        let global = self.builder.build_global_string_ptr(s, "str_lit").unwrap();
        let ptr_val = global.as_pointer_value();
        let len_val = self.context.i64_type().const_int(s.len() as u64, false);

        // Build the {ptr, len} struct
        let str_val = str_ty.const_zero();
        let str_val = self
            .builder
            .build_insert_value(str_val, ptr_val, 0, "str_ptr")
            .unwrap();
        let str_val = self
            .builder
            .build_insert_value(str_val, len_val, 1, "str_len")
            .unwrap();
        str_val.into_struct_value().into()
    }

    // ------------------------------------------------------------------
    // print/println builtins
    // ------------------------------------------------------------------

    pub(super) fn compile_print_call(
        &mut self,
        callee: &str,
        args: &[Expr],
        function: &FunctionValue<'ctx>,
    ) -> Result<(), Vec<CompileError>> {
        let is_println = callee == "println";

        for arg in args {
            let arg_ty = self.infer_expr_type(arg);
            let val = self.compile_expr(arg, function)?;

            match &arg_ty {
                NyType::I32 => {
                    let printf_fn = self.get_or_declare_printf();
                    let fmt = self
                        .builder
                        .build_global_string_ptr("%d", "fmt_i32")
                        .unwrap();
                    self.builder
                        .build_call(
                            printf_fn,
                            &[fmt.as_pointer_value().into(), val.unwrap().into()],
                            "",
                        )
                        .unwrap();
                }
                NyType::I64 => {
                    let printf_fn = self.get_or_declare_printf();
                    let fmt = self
                        .builder
                        .build_global_string_ptr("%ld", "fmt_i64")
                        .unwrap();
                    self.builder
                        .build_call(
                            printf_fn,
                            &[fmt.as_pointer_value().into(), val.unwrap().into()],
                            "",
                        )
                        .unwrap();
                }
                t if t.is_integer() => {
                    // All other integer types: print as %ld after sign/zero extending to i64
                    let printf_fn = self.get_or_declare_printf();
                    let fmt = self
                        .builder
                        .build_global_string_ptr("%ld", "fmt_int")
                        .unwrap();
                    let int_val = val.unwrap().into_int_value();
                    let i64_val = if t.is_signed() {
                        self.builder
                            .build_int_s_extend(int_val, self.context.i64_type(), "sext")
                            .unwrap()
                    } else {
                        self.builder
                            .build_int_z_extend(int_val, self.context.i64_type(), "zext")
                            .unwrap()
                    };
                    self.builder
                        .build_call(
                            printf_fn,
                            &[fmt.as_pointer_value().into(), i64_val.into()],
                            "",
                        )
                        .unwrap();
                }
                NyType::F64 | NyType::F32 => {
                    let printf_fn = self.get_or_declare_printf();
                    let fmt = self
                        .builder
                        .build_global_string_ptr("%f", "fmt_float")
                        .unwrap();
                    // Ensure f32 is promoted to f64 for printf (varargs convention)
                    let float_val = if arg_ty == NyType::F32 {
                        let f32_val = val.unwrap().into_float_value();
                        self.builder
                            .build_float_ext(f32_val, self.context.f64_type(), "fpext")
                            .unwrap()
                            .into()
                    } else {
                        val.unwrap()
                    };
                    self.builder
                        .build_call(
                            printf_fn,
                            &[fmt.as_pointer_value().into(), float_val.into()],
                            "",
                        )
                        .unwrap();
                }
                NyType::Bool => {
                    // Print "true" or "false"
                    let bool_val = val.unwrap().into_int_value();
                    let true_str = self
                        .builder
                        .build_global_string_ptr("true", "true_str")
                        .unwrap();
                    let false_str = self
                        .builder
                        .build_global_string_ptr("false", "false_str")
                        .unwrap();
                    let selected = self
                        .builder
                        .build_select(
                            bool_val,
                            true_str.as_pointer_value(),
                            false_str.as_pointer_value(),
                            "bool_str",
                        )
                        .unwrap();
                    let printf_fn = self.get_or_declare_printf();
                    let fmt = self
                        .builder
                        .build_global_string_ptr("%s", "fmt_bool")
                        .unwrap();
                    self.builder
                        .build_call(
                            printf_fn,
                            &[fmt.as_pointer_value().into(), selected.into()],
                            "",
                        )
                        .unwrap();
                }
                NyType::Str => {
                    // str is {ptr, len} — use write(1, ptr, len)
                    let write_fn = self.get_or_declare_write();
                    let str_val = val.unwrap().into_struct_value();
                    let ptr = self
                        .builder
                        .build_extract_value(str_val, 0, "str_ptr")
                        .unwrap();
                    let len = self
                        .builder
                        .build_extract_value(str_val, 1, "str_len")
                        .unwrap();
                    let fd = self.context.i32_type().const_int(1, false); // stdout
                    self.builder
                        .build_call(write_fn, &[fd.into(), ptr.into(), len.into()], "")
                        .unwrap();
                }
                NyType::Struct { name, fields } => {
                    // Print struct as StructName { field1: val1, field2: val2, ... }
                    let printf_fn = self.get_or_declare_printf();
                    let struct_val = val.unwrap().into_struct_value();

                    // Print "StructName { "
                    let header = self
                        .builder
                        .build_global_string_ptr(&format!("{} {{ ", name), "struct_header")
                        .unwrap();
                    let fmt_s = self.builder.build_global_string_ptr("%s", "fmt_s").unwrap();
                    self.builder
                        .build_call(
                            printf_fn,
                            &[
                                fmt_s.as_pointer_value().into(),
                                header.as_pointer_value().into(),
                            ],
                            "",
                        )
                        .unwrap();

                    for (i, (fname, ftype)) in fields.iter().enumerate() {
                        let field_val = self
                            .builder
                            .build_extract_value(struct_val, i as u32, fname)
                            .unwrap();

                        // Print "field_name: "
                        let prefix = if i > 0 {
                            format!(", {}: ", fname)
                        } else {
                            format!("{}: ", fname)
                        };
                        let prefix_str = self
                            .builder
                            .build_global_string_ptr(&prefix, "field_prefix")
                            .unwrap();
                        self.builder
                            .build_call(
                                printf_fn,
                                &[
                                    fmt_s.as_pointer_value().into(),
                                    prefix_str.as_pointer_value().into(),
                                ],
                                "",
                            )
                            .unwrap();

                        // Print the field value based on its type
                        match ftype {
                            NyType::I32 => {
                                let fmt_d =
                                    self.builder.build_global_string_ptr("%d", "fmt_d").unwrap();
                                self.builder
                                    .build_call(
                                        printf_fn,
                                        &[fmt_d.as_pointer_value().into(), field_val.into()],
                                        "",
                                    )
                                    .unwrap();
                            }
                            NyType::I64 => {
                                let fmt_ld = self
                                    .builder
                                    .build_global_string_ptr("%ld", "fmt_ld")
                                    .unwrap();
                                self.builder
                                    .build_call(
                                        printf_fn,
                                        &[fmt_ld.as_pointer_value().into(), field_val.into()],
                                        "",
                                    )
                                    .unwrap();
                            }
                            NyType::F64 | NyType::F32 => {
                                let fmt_f =
                                    self.builder.build_global_string_ptr("%f", "fmt_f").unwrap();
                                self.builder
                                    .build_call(
                                        printf_fn,
                                        &[fmt_f.as_pointer_value().into(), field_val.into()],
                                        "",
                                    )
                                    .unwrap();
                            }
                            NyType::Bool => {
                                let bool_v = field_val.into_int_value();
                                let ts = self.builder.build_global_string_ptr("true", "t").unwrap();
                                let fs =
                                    self.builder.build_global_string_ptr("false", "f").unwrap();
                                let sel = self
                                    .builder
                                    .build_select(
                                        bool_v,
                                        ts.as_pointer_value(),
                                        fs.as_pointer_value(),
                                        "bs",
                                    )
                                    .unwrap();
                                let fmt_bs = self
                                    .builder
                                    .build_global_string_ptr("%s", "fmt_bs")
                                    .unwrap();
                                self.builder
                                    .build_call(
                                        printf_fn,
                                        &[fmt_bs.as_pointer_value().into(), sel.into()],
                                        "",
                                    )
                                    .unwrap();
                            }
                            _ => {
                                // Fallback: print as integer
                                let fmt_d = self
                                    .builder
                                    .build_global_string_ptr("%d", "fmt_d_fb")
                                    .unwrap();
                                self.builder
                                    .build_call(
                                        printf_fn,
                                        &[fmt_d.as_pointer_value().into(), field_val.into()],
                                        "",
                                    )
                                    .unwrap();
                            }
                        }
                    }

                    // Print " }"
                    let footer = self
                        .builder
                        .build_global_string_ptr(" }", "struct_footer")
                        .unwrap();
                    self.builder
                        .build_call(
                            printf_fn,
                            &[
                                fmt_s.as_pointer_value().into(),
                                footer.as_pointer_value().into(),
                            ],
                            "",
                        )
                        .unwrap();
                }
                NyType::Enum { name, variants } => {
                    // Print enum variant name using a switch on the discriminant
                    let printf_fn = self.get_or_declare_printf();
                    let fmt_s = self
                        .builder
                        .build_global_string_ptr("%s", "fmt_enum")
                        .unwrap();
                    let disc_val = val.unwrap().into_int_value();

                    // Extract variant names for printing
                    let variant_names: Vec<String> =
                        variants.iter().map(|(name, _)| name.clone()).collect();

                    // Capture the current block before creating new ones
                    let origin_bb = self.builder.get_insert_block().unwrap();

                    let merge_bb = self
                        .context
                        .append_basic_block(*function, "enum_print_merge");
                    let default_bb = self
                        .context
                        .append_basic_block(*function, "enum_print_default");

                    // Create basic blocks for each variant
                    let variant_bbs: Vec<BasicBlock> = variant_names
                        .iter()
                        .enumerate()
                        .map(|(i, _)| {
                            self.context
                                .append_basic_block(*function, &format!("enum_print_{}", i))
                        })
                        .collect();

                    // Build default block: print "EnumName::?" for unknown discriminant
                    self.builder.position_at_end(default_bb);
                    let unknown_str = self
                        .builder
                        .build_global_string_ptr(&format!("{}::?", name), "enum_unknown")
                        .unwrap();
                    self.builder
                        .build_call(
                            printf_fn,
                            &[
                                fmt_s.as_pointer_value().into(),
                                unknown_str.as_pointer_value().into(),
                            ],
                            "",
                        )
                        .unwrap();
                    self.builder.build_unconditional_branch(merge_bb).unwrap();

                    // Build each variant print block
                    for (i, variant_name) in variant_names.iter().enumerate() {
                        self.builder.position_at_end(variant_bbs[i]);
                        let var_str = self
                            .builder
                            .build_global_string_ptr(
                                &format!("{}::{}", name, variant_name),
                                &format!("enum_var_{}", i),
                            )
                            .unwrap();
                        self.builder
                            .build_call(
                                printf_fn,
                                &[
                                    fmt_s.as_pointer_value().into(),
                                    var_str.as_pointer_value().into(),
                                ],
                                "",
                            )
                            .unwrap();
                        self.builder.build_unconditional_branch(merge_bb).unwrap();
                    }

                    // Go back to the origin block and emit the switch
                    self.builder.position_at_end(origin_bb);
                    let cases: Vec<(inkwell::values::IntValue, BasicBlock)> = variant_names
                        .iter()
                        .enumerate()
                        .map(|(i, _)| {
                            (
                                self.context.i32_type().const_int(i as u64, false),
                                variant_bbs[i],
                            )
                        })
                        .collect();
                    self.builder
                        .build_switch(disc_val, default_bb, &cases)
                        .unwrap();

                    self.builder.position_at_end(merge_bb);
                }
                NyType::Tuple(ref elem_types) => {
                    // Print tuple as (elem1, elem2, ...)
                    let printf_fn = self.get_or_declare_printf();
                    let tuple_val = val.unwrap().into_struct_value();

                    // Print opening paren
                    let open_paren = self
                        .builder
                        .build_global_string_ptr("(", "tuple_open")
                        .unwrap();
                    let fmt_s = self
                        .builder
                        .build_global_string_ptr("%s", "fmt_s_tuple")
                        .unwrap();
                    self.builder
                        .build_call(
                            printf_fn,
                            &[
                                fmt_s.as_pointer_value().into(),
                                open_paren.as_pointer_value().into(),
                            ],
                            "",
                        )
                        .unwrap();

                    for (i, et) in elem_types.iter().enumerate() {
                        if i > 0 {
                            let sep = self
                                .builder
                                .build_global_string_ptr(", ", "tuple_sep")
                                .unwrap();
                            self.builder
                                .build_call(
                                    printf_fn,
                                    &[
                                        fmt_s.as_pointer_value().into(),
                                        sep.as_pointer_value().into(),
                                    ],
                                    "",
                                )
                                .unwrap();
                        }

                        let elem_val = self
                            .builder
                            .build_extract_value(tuple_val, i as u32, &format!("tup_el_{}", i))
                            .unwrap();

                        match et {
                            NyType::I32 => {
                                let fmt_d = self
                                    .builder
                                    .build_global_string_ptr("%d", "fmt_d_tup")
                                    .unwrap();
                                self.builder
                                    .build_call(
                                        printf_fn,
                                        &[fmt_d.as_pointer_value().into(), elem_val.into()],
                                        "",
                                    )
                                    .unwrap();
                            }
                            NyType::I64 => {
                                let fmt_ld = self
                                    .builder
                                    .build_global_string_ptr("%ld", "fmt_ld_tup")
                                    .unwrap();
                                self.builder
                                    .build_call(
                                        printf_fn,
                                        &[fmt_ld.as_pointer_value().into(), elem_val.into()],
                                        "",
                                    )
                                    .unwrap();
                            }
                            NyType::F64 | NyType::F32 => {
                                let fmt_f = self
                                    .builder
                                    .build_global_string_ptr("%f", "fmt_f_tup")
                                    .unwrap();
                                let float_val = if *et == NyType::F32 {
                                    let f32_val = elem_val.into_float_value();
                                    self.builder
                                        .build_float_ext(
                                            f32_val,
                                            self.context.f64_type(),
                                            "fpext_tup",
                                        )
                                        .unwrap()
                                        .into()
                                } else {
                                    elem_val
                                };
                                self.builder
                                    .build_call(
                                        printf_fn,
                                        &[fmt_f.as_pointer_value().into(), float_val.into()],
                                        "",
                                    )
                                    .unwrap();
                            }
                            NyType::Bool => {
                                let bool_v = elem_val.into_int_value();
                                let ts = self
                                    .builder
                                    .build_global_string_ptr("true", "t_tup")
                                    .unwrap();
                                let fs = self
                                    .builder
                                    .build_global_string_ptr("false", "f_tup")
                                    .unwrap();
                                let sel = self
                                    .builder
                                    .build_select(
                                        bool_v,
                                        ts.as_pointer_value(),
                                        fs.as_pointer_value(),
                                        "bs_tup",
                                    )
                                    .unwrap();
                                self.builder
                                    .build_call(
                                        printf_fn,
                                        &[fmt_s.as_pointer_value().into(), sel.into()],
                                        "",
                                    )
                                    .unwrap();
                            }
                            NyType::Str => {
                                let write_fn = self.get_or_declare_write();
                                let sv = elem_val.into_struct_value();
                                let ptr = self
                                    .builder
                                    .build_extract_value(sv, 0, "tup_str_ptr")
                                    .unwrap();
                                let len = self
                                    .builder
                                    .build_extract_value(sv, 1, "tup_str_len")
                                    .unwrap();
                                let fd = self.context.i32_type().const_int(1, false);
                                self.builder
                                    .build_call(write_fn, &[fd.into(), ptr.into(), len.into()], "")
                                    .unwrap();
                            }
                            _ => {
                                let fmt_d = self
                                    .builder
                                    .build_global_string_ptr("%d", "fmt_d_tup_fb")
                                    .unwrap();
                                self.builder
                                    .build_call(
                                        printf_fn,
                                        &[fmt_d.as_pointer_value().into(), elem_val.into()],
                                        "",
                                    )
                                    .unwrap();
                            }
                        }
                    }

                    // Print closing paren
                    let close_paren = self
                        .builder
                        .build_global_string_ptr(")", "tuple_close")
                        .unwrap();
                    self.builder
                        .build_call(
                            printf_fn,
                            &[
                                fmt_s.as_pointer_value().into(),
                                close_paren.as_pointer_value().into(),
                            ],
                            "",
                        )
                        .unwrap();
                }
                NyType::Vec(ref elem_ty) => {
                    // Print Vec as [elem1, elem2, ...]
                    let printf_fn = self.get_or_declare_printf();
                    let vec_val = val.unwrap().into_struct_value();
                    let data_ptr = self
                        .builder
                        .build_extract_value(vec_val, 0, "vec_data")
                        .unwrap()
                        .into_pointer_value();
                    let len = self
                        .builder
                        .build_extract_value(vec_val, 1, "vec_len")
                        .unwrap()
                        .into_int_value();

                    let open = self
                        .builder
                        .build_global_string_ptr("[", "vec_open")
                        .unwrap();
                    let fmt_s = self
                        .builder
                        .build_global_string_ptr("%s", "fmt_s_vec")
                        .unwrap();
                    self.builder
                        .build_call(
                            printf_fn,
                            &[
                                fmt_s.as_pointer_value().into(),
                                open.as_pointer_value().into(),
                            ],
                            "",
                        )
                        .unwrap();

                    // Loop to print each element
                    let i_alloca = self
                        .builder
                        .build_alloca(self.context.i64_type(), "vec_print_i")
                        .unwrap();
                    self.builder
                        .build_store(i_alloca, self.context.i64_type().const_zero())
                        .unwrap();

                    let cond_bb = self.context.append_basic_block(*function, "vec_print_cond");
                    let body_bb = self.context.append_basic_block(*function, "vec_print_body");
                    let done_bb = self.context.append_basic_block(*function, "vec_print_done");

                    self.builder.build_unconditional_branch(cond_bb).unwrap();

                    self.builder.position_at_end(cond_bb);
                    let i = self
                        .builder
                        .build_load(self.context.i64_type(), i_alloca, "i")
                        .unwrap()
                        .into_int_value();
                    let cmp = self
                        .builder
                        .build_int_compare(IntPredicate::ULT, i, len, "vec_cmp")
                        .unwrap();
                    self.builder
                        .build_conditional_branch(cmp, body_bb, done_bb)
                        .unwrap();

                    self.builder.position_at_end(body_bb);
                    // Print separator
                    let zero = self.context.i64_type().const_zero();
                    let not_first = self
                        .builder
                        .build_int_compare(IntPredicate::UGT, i, zero, "not_first")
                        .unwrap();
                    let sep = self
                        .builder
                        .build_global_string_ptr(", ", "vec_sep")
                        .unwrap();
                    let empty = self
                        .builder
                        .build_global_string_ptr("", "vec_empty")
                        .unwrap();
                    let sep_str = self
                        .builder
                        .build_select(
                            not_first,
                            sep.as_pointer_value(),
                            empty.as_pointer_value(),
                            "sep_sel",
                        )
                        .unwrap();
                    self.builder
                        .build_call(
                            printf_fn,
                            &[fmt_s.as_pointer_value().into(), sep_str.into()],
                            "",
                        )
                        .unwrap();

                    // Print element
                    let elem_llvm = ny_to_llvm(self.context, elem_ty);
                    let elem_ptr = unsafe {
                        self.builder
                            .build_in_bounds_gep(elem_llvm, data_ptr, &[i], "vec_ep")
                            .unwrap()
                    };
                    let elem_val = self
                        .builder
                        .build_load(elem_llvm, elem_ptr, "vec_e")
                        .unwrap();
                    let fmt_d = self
                        .builder
                        .build_global_string_ptr("%d", "fmt_d_vec")
                        .unwrap();
                    self.builder
                        .build_call(
                            printf_fn,
                            &[fmt_d.as_pointer_value().into(), elem_val.into()],
                            "",
                        )
                        .unwrap();

                    // i++
                    let next_i = self
                        .builder
                        .build_int_add(i, self.context.i64_type().const_int(1, false), "next_i")
                        .unwrap();
                    self.builder.build_store(i_alloca, next_i).unwrap();
                    self.builder.build_unconditional_branch(cond_bb).unwrap();

                    self.builder.position_at_end(done_bb);
                    let close = self
                        .builder
                        .build_global_string_ptr("]", "vec_close")
                        .unwrap();
                    self.builder
                        .build_call(
                            printf_fn,
                            &[
                                fmt_s.as_pointer_value().into(),
                                close.as_pointer_value().into(),
                            ],
                            "",
                        )
                        .unwrap();
                }
                _ => {
                    // Fallback: try to print as i32
                    if let Some(v) = val {
                        let printf_fn = self.get_or_declare_printf();
                        let fmt = self
                            .builder
                            .build_global_string_ptr("%d", "fmt_fallback")
                            .unwrap();
                        self.builder
                            .build_call(printf_fn, &[fmt.as_pointer_value().into(), v.into()], "")
                            .unwrap();
                    }
                }
            }
        }

        // For println, print a trailing newline
        if is_println {
            let printf_fn = self.get_or_declare_printf();
            let newline = self
                .builder
                .build_global_string_ptr("\n", "newline")
                .unwrap();
            let fmt_s = self
                .builder
                .build_global_string_ptr("%s", "fmt_nl")
                .unwrap();
            self.builder
                .build_call(
                    printf_fn,
                    &[
                        fmt_s.as_pointer_value().into(),
                        newline.as_pointer_value().into(),
                    ],
                    "",
                )
                .unwrap();
        }

        // Flush stdout after every print/println to avoid buffering issues
        let fflush_fn = self.get_or_declare_fflush();
        let null_ptr = self.context.ptr_type(AddressSpace::default()).const_null();
        self.builder
            .build_call(fflush_fn, &[null_ptr.into()], "")
            .unwrap();

        Ok(())
    }
}
