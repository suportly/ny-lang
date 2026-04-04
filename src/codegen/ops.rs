use inkwell::values::BasicValueEnum;
use inkwell::{FloatPredicate, IntPredicate};

use crate::common::{CompileError, Span};
use crate::parser::ast::{BinOp, UnaryOp};

use super::types::str_type;
use super::CodeGen;

impl<'ctx> CodeGen<'ctx> {
    // ------------------------------------------------------------------
    // Binary and unary operations
    // ------------------------------------------------------------------

    pub(super) fn compile_binop(
        &self,
        op: BinOp,
        lhs: BasicValueEnum<'ctx>,
        rhs: BasicValueEnum<'ctx>,
    ) -> Result<BasicValueEnum<'ctx>, Vec<CompileError>> {
        // Handle string operations (both operands are {ptr, len} struct values)
        if lhs.is_struct_value() && rhs.is_struct_value() {
            let l = lhs.into_struct_value();
            let r = rhs.into_struct_value();

            match op {
                BinOp::Add => {
                    // String concatenation
                    let l_ptr = self.builder.build_extract_value(l, 0, "l_ptr").unwrap();
                    let l_len = self
                        .builder
                        .build_extract_value(l, 1, "l_len")
                        .unwrap()
                        .into_int_value();
                    let r_ptr = self.builder.build_extract_value(r, 0, "r_ptr").unwrap();
                    let r_len = self
                        .builder
                        .build_extract_value(r, 1, "r_len")
                        .unwrap()
                        .into_int_value();

                    let total_len = self
                        .builder
                        .build_int_add(l_len, r_len, "total_len")
                        .unwrap();

                    // malloc(total_len)
                    let malloc_fn = self.get_or_declare_malloc();
                    let result_ptr = self
                        .builder
                        .build_call(malloc_fn, &[total_len.into()], "concat_buf")
                        .unwrap()
                        .try_as_basic_value()
                        .basic()
                        .unwrap()
                        .into_pointer_value();

                    // memcpy(result_ptr, l_ptr, l_len)
                    let memcpy_fn = self.get_or_declare_memcpy();
                    self.builder
                        .build_call(
                            memcpy_fn,
                            &[result_ptr.into(), l_ptr.into(), l_len.into()],
                            "",
                        )
                        .unwrap();

                    // memcpy(result_ptr + l_len, r_ptr, r_len)
                    let i8_ty = self.context.i8_type();
                    let dest_offset = unsafe {
                        self.builder
                            .build_in_bounds_gep(i8_ty, result_ptr, &[l_len], "concat_dest")
                            .unwrap()
                    };
                    self.builder
                        .build_call(
                            memcpy_fn,
                            &[dest_offset.into(), r_ptr.into(), r_len.into()],
                            "",
                        )
                        .unwrap();

                    // Build {result_ptr, total_len} struct
                    let str_ty = str_type(self.context);
                    let str_val = str_ty.const_zero();
                    let str_val = self
                        .builder
                        .build_insert_value(str_val, result_ptr, 0, "cat_ptr")
                        .unwrap();
                    let str_val = self
                        .builder
                        .build_insert_value(str_val, total_len, 1, "cat_len")
                        .unwrap();
                    return Ok(str_val.into_struct_value().into());
                }
                BinOp::Eq | BinOp::Ne => {
                    // String comparison
                    let l_ptr = self.builder.build_extract_value(l, 0, "l_ptr").unwrap();
                    let l_len = self
                        .builder
                        .build_extract_value(l, 1, "l_len")
                        .unwrap()
                        .into_int_value();
                    let r_ptr = self.builder.build_extract_value(r, 0, "r_ptr").unwrap();
                    let r_len = self
                        .builder
                        .build_extract_value(r, 1, "r_len")
                        .unwrap()
                        .into_int_value();

                    // Compare lengths
                    let len_eq = self
                        .builder
                        .build_int_compare(IntPredicate::EQ, l_len, r_len, "len_eq")
                        .unwrap();

                    // If lengths are equal, compare contents with memcmp
                    let memcmp_fn = self.get_or_declare_memcmp();
                    let cmp_result = self
                        .builder
                        .build_call(
                            memcmp_fn,
                            &[l_ptr.into(), r_ptr.into(), l_len.into()],
                            "memcmp_res",
                        )
                        .unwrap()
                        .try_as_basic_value()
                        .basic()
                        .unwrap()
                        .into_int_value();
                    let cmp_eq = self
                        .builder
                        .build_int_compare(
                            IntPredicate::EQ,
                            cmp_result,
                            self.context.i32_type().const_zero(),
                            "cmp_eq",
                        )
                        .unwrap();

                    // Final result: lengths equal AND contents equal
                    let str_eq = self.builder.build_and(len_eq, cmp_eq, "str_eq").unwrap();

                    if op == BinOp::Ne {
                        let negated = self.builder.build_not(str_eq, "str_ne").unwrap();
                        return Ok(negated.into());
                    }
                    return Ok(str_eq.into());
                }
                _ => {
                    // Try operator overloading: look for TypeName_opname function
                    let op_method = match op {
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
                        _ => {
                            return Err(vec![CompileError::type_error(
                                "unsupported binary operation on struct types".to_string(),
                                Span::empty(0),
                            )]);
                        }
                    };

                    // Try to find operator method in registered functions
                    // Methods are registered as TypeName_methodname
                    for (name, (func, param_types, ret_ty)) in &self.functions {
                        if name.ends_with(&format!("_{}", op_method)) && param_types.len() == 2 {
                            let result = self
                                .builder
                                .build_call(*func, &[lhs.into(), rhs.into()], "op_result")
                                .unwrap();
                            if *ret_ty == crate::common::NyType::Bool {
                                return Ok(result
                                    .try_as_basic_value()
                                    .basic()
                                    .unwrap());
                            }
                            return Ok(result
                                .try_as_basic_value()
                                .basic()
                                .unwrap());
                        }
                    }

                    return Err(vec![CompileError::type_error(
                        format!("no operator '{}' defined for struct type", op_method),
                        Span::empty(0),
                    )]);
                }
            }
        }

        // Pointer arithmetic: ptr + int or ptr - int
        if lhs.is_pointer_value() && rhs.is_int_value() {
            let ptr = lhs.into_pointer_value();
            let offset = rhs.into_int_value();
            let i8_ty = self.context.i8_type();
            match op {
                BinOp::Add => {
                    let offset_i64 = self
                        .builder
                        .build_int_s_extend_or_bit_cast(offset, self.context.i64_type(), "ptr_off")
                        .unwrap();
                    let result = unsafe {
                        self.builder
                            .build_in_bounds_gep(i8_ty, ptr, &[offset_i64], "ptr_add")
                            .unwrap()
                    };
                    return Ok(result.into());
                }
                BinOp::Sub => {
                    let neg_offset = self.builder.build_int_neg(offset, "neg_off").unwrap();
                    let neg_i64 = self
                        .builder
                        .build_int_s_extend_or_bit_cast(
                            neg_offset,
                            self.context.i64_type(),
                            "ptr_neg",
                        )
                        .unwrap();
                    let result = unsafe {
                        self.builder
                            .build_in_bounds_gep(i8_ty, ptr, &[neg_i64], "ptr_sub")
                            .unwrap()
                    };
                    return Ok(result.into());
                }
                _ => {
                    return Err(vec![CompileError::type_error(
                        "only + and - are supported for pointer arithmetic".to_string(),
                        Span::empty(0),
                    )]);
                }
            }
        }

        // Pointer comparison: ptr == nil, ptr != nil, ptr == ptr
        if lhs.is_pointer_value() && rhs.is_pointer_value() {
            let l = lhs.into_pointer_value();
            let r = rhs.into_pointer_value();
            let result = match op {
                BinOp::Eq => self.builder.build_int_compare(
                    IntPredicate::EQ,
                    self.builder.build_ptr_to_int(l, self.context.i64_type(), "l_int").unwrap(),
                    self.builder.build_ptr_to_int(r, self.context.i64_type(), "r_int").unwrap(),
                    "ptr_eq",
                ).unwrap(),
                BinOp::Ne => self.builder.build_int_compare(
                    IntPredicate::NE,
                    self.builder.build_ptr_to_int(l, self.context.i64_type(), "l_int").unwrap(),
                    self.builder.build_ptr_to_int(r, self.context.i64_type(), "r_int").unwrap(),
                    "ptr_ne",
                ).unwrap(),
                _ => {
                    return Err(vec![CompileError::type_error(
                        "only == and != are supported for pointer comparison".to_string(),
                        Span::empty(0),
                    )]);
                }
            };
            return Ok(result.into());
        }

        if lhs.is_int_value() && rhs.is_int_value() {
            let mut l = lhs.into_int_value();
            let mut r = rhs.into_int_value();

            // Auto-widen: if operands have different bit widths, extend the narrower one
            let l_bits = l.get_type().get_bit_width();
            let r_bits = r.get_type().get_bit_width();
            if l_bits < r_bits {
                l = self
                    .builder
                    .build_int_s_extend(l, r.get_type(), "widen_l")
                    .unwrap();
            } else if r_bits < l_bits {
                r = self
                    .builder
                    .build_int_s_extend(r, l.get_type(), "widen_r")
                    .unwrap();
            }
            let result: BasicValueEnum = match op {
                BinOp::Add => self.builder.build_int_add(l, r, "add").unwrap().into(),
                BinOp::Sub => self.builder.build_int_sub(l, r, "sub").unwrap().into(),
                BinOp::Mul => self.builder.build_int_mul(l, r, "mul").unwrap().into(),
                BinOp::Div => self
                    .builder
                    .build_int_signed_div(l, r, "div")
                    .unwrap()
                    .into(),
                BinOp::Mod => self
                    .builder
                    .build_int_signed_rem(l, r, "rem")
                    .unwrap()
                    .into(),
                BinOp::Eq => self
                    .builder
                    .build_int_compare(IntPredicate::EQ, l, r, "eq")
                    .unwrap()
                    .into(),
                BinOp::Ne => self
                    .builder
                    .build_int_compare(IntPredicate::NE, l, r, "ne")
                    .unwrap()
                    .into(),
                BinOp::Lt => self
                    .builder
                    .build_int_compare(IntPredicate::SLT, l, r, "lt")
                    .unwrap()
                    .into(),
                BinOp::Gt => self
                    .builder
                    .build_int_compare(IntPredicate::SGT, l, r, "gt")
                    .unwrap()
                    .into(),
                BinOp::Le => self
                    .builder
                    .build_int_compare(IntPredicate::SLE, l, r, "le")
                    .unwrap()
                    .into(),
                BinOp::Ge => self
                    .builder
                    .build_int_compare(IntPredicate::SGE, l, r, "ge")
                    .unwrap()
                    .into(),
                BinOp::And => self.builder.build_and(l, r, "and").unwrap().into(),
                BinOp::Or => self.builder.build_or(l, r, "or").unwrap().into(),
                BinOp::BitAnd => self.builder.build_and(l, r, "bitand").unwrap().into(),
                BinOp::BitOr => self.builder.build_or(l, r, "bitor").unwrap().into(),
                BinOp::BitXor => self.builder.build_xor(l, r, "bitxor").unwrap().into(),
                BinOp::Shl => self.builder.build_left_shift(l, r, "shl").unwrap().into(),
                BinOp::Shr => self
                    .builder
                    .build_right_shift(l, r, true, "shr")
                    .unwrap()
                    .into(),
            };
            Ok(result)
        } else if lhs.is_float_value() && rhs.is_float_value() {
            let mut l = lhs.into_float_value();
            let mut r = rhs.into_float_value();

            // Auto-widen: f32 → f64 if mixed
            if l.get_type() != r.get_type() {
                if l.get_type() == self.context.f32_type() {
                    l = self
                        .builder
                        .build_float_ext(l, self.context.f64_type(), "widen_fl")
                        .unwrap();
                } else {
                    r = self
                        .builder
                        .build_float_ext(r, self.context.f64_type(), "widen_fr")
                        .unwrap();
                }
            }
            let result: BasicValueEnum = match op {
                BinOp::Add => self.builder.build_float_add(l, r, "fadd").unwrap().into(),
                BinOp::Sub => self.builder.build_float_sub(l, r, "fsub").unwrap().into(),
                BinOp::Mul => self.builder.build_float_mul(l, r, "fmul").unwrap().into(),
                BinOp::Div => self.builder.build_float_div(l, r, "fdiv").unwrap().into(),
                BinOp::Mod => self.builder.build_float_rem(l, r, "frem").unwrap().into(),
                BinOp::Eq => self
                    .builder
                    .build_float_compare(FloatPredicate::OEQ, l, r, "feq")
                    .unwrap()
                    .into(),
                BinOp::Ne => self
                    .builder
                    .build_float_compare(FloatPredicate::ONE, l, r, "fne")
                    .unwrap()
                    .into(),
                BinOp::Lt => self
                    .builder
                    .build_float_compare(FloatPredicate::OLT, l, r, "flt")
                    .unwrap()
                    .into(),
                BinOp::Gt => self
                    .builder
                    .build_float_compare(FloatPredicate::OGT, l, r, "fgt")
                    .unwrap()
                    .into(),
                BinOp::Le => self
                    .builder
                    .build_float_compare(FloatPredicate::OLE, l, r, "fle")
                    .unwrap()
                    .into(),
                BinOp::Ge => self
                    .builder
                    .build_float_compare(FloatPredicate::OGE, l, r, "fge")
                    .unwrap()
                    .into(),
                BinOp::And
                | BinOp::Or
                | BinOp::BitAnd
                | BinOp::BitOr
                | BinOp::BitXor
                | BinOp::Shl
                | BinOp::Shr => {
                    unreachable!("logical/bitwise ops on floats")
                }
            };
            Ok(result)
        } else if (lhs.is_int_value() && rhs.is_float_value())
            || (lhs.is_float_value() && rhs.is_int_value())
        {
            // Mixed int + float: promote int to float
            let (fl, fr) = if lhs.is_float_value() {
                let f = lhs.into_float_value();
                let i = rhs.into_int_value();
                let promoted = self
                    .builder
                    .build_signed_int_to_float(i, f.get_type(), "i2f")
                    .unwrap();
                (f, promoted)
            } else {
                let i = lhs.into_int_value();
                let f = rhs.into_float_value();
                let promoted = self
                    .builder
                    .build_signed_int_to_float(i, f.get_type(), "i2f")
                    .unwrap();
                (promoted, f)
            };
            let result: BasicValueEnum = match op {
                BinOp::Add => self.builder.build_float_add(fl, fr, "fadd").unwrap().into(),
                BinOp::Sub => self.builder.build_float_sub(fl, fr, "fsub").unwrap().into(),
                BinOp::Mul => self.builder.build_float_mul(fl, fr, "fmul").unwrap().into(),
                BinOp::Div => self.builder.build_float_div(fl, fr, "fdiv").unwrap().into(),
                _ => {
                    return Err(vec![CompileError::type_error(
                        "unsupported operation on mixed int/float".to_string(),
                        Span::empty(0),
                    )]);
                }
            };
            Ok(result)
        } else if lhs.is_vector_value() && rhs.is_vector_value() {
            // SIMD vector arithmetic
            let l = lhs.into_vector_value();
            let r = rhs.into_vector_value();
            // Check if the element type is float or int
            let elem_kind = l.get_type().get_element_type();
            if elem_kind.is_float_type() {
                let result: BasicValueEnum = match op {
                    BinOp::Add => self.builder.build_float_add(l, r, "vadd").unwrap().into(),
                    BinOp::Sub => self.builder.build_float_sub(l, r, "vsub").unwrap().into(),
                    BinOp::Mul => self.builder.build_float_mul(l, r, "vmul").unwrap().into(),
                    BinOp::Div => self.builder.build_float_div(l, r, "vdiv").unwrap().into(),
                    _ => {
                        return Err(vec![CompileError::type_error(
                            "unsupported SIMD operation".to_string(),
                            Span::empty(0),
                        )]);
                    }
                };
                Ok(result)
            } else {
                let result: BasicValueEnum = match op {
                    BinOp::Add => self.builder.build_int_add(l, r, "vadd").unwrap().into(),
                    BinOp::Sub => self.builder.build_int_sub(l, r, "vsub").unwrap().into(),
                    BinOp::Mul => self.builder.build_int_mul(l, r, "vmul").unwrap().into(),
                    _ => {
                        return Err(vec![CompileError::type_error(
                            "unsupported SIMD operation".to_string(),
                            Span::empty(0),
                        )]);
                    }
                };
                Ok(result)
            }
        } else {
            Err(vec![CompileError::type_error(
                "binary operation on incompatible types".to_string(),
                Span::empty(0),
            )])
        }
    }

    pub(super) fn compile_unaryop(
        &self,
        op: UnaryOp,
        operand: BasicValueEnum<'ctx>,
    ) -> Result<BasicValueEnum<'ctx>, Vec<CompileError>> {
        match op {
            UnaryOp::Neg => {
                if operand.is_int_value() {
                    Ok(self
                        .builder
                        .build_int_neg(operand.into_int_value(), "neg")
                        .unwrap()
                        .into())
                } else {
                    Ok(self
                        .builder
                        .build_float_neg(operand.into_float_value(), "fneg")
                        .unwrap()
                        .into())
                }
            }
            UnaryOp::Not => Ok(self
                .builder
                .build_not(operand.into_int_value(), "not")
                .unwrap()
                .into()),
            UnaryOp::BitNot => Ok(self
                .builder
                .build_not(operand.into_int_value(), "bitnot")
                .unwrap()
                .into()),
        }
    }
}
