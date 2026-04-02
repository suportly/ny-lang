use inkwell::types::BasicTypeEnum;
use inkwell::values::BasicValueEnum;

use crate::common::{CompileError, NyType, Span};

use super::CodeGen;

impl<'ctx> CodeGen<'ctx> {
    pub(super) fn compile_cast(
        &self,
        val: BasicValueEnum<'ctx>,
        source_ty: &NyType,
        target_ty: &NyType,
        target_llvm: BasicTypeEnum<'ctx>,
    ) -> Result<BasicValueEnum<'ctx>, Vec<CompileError>> {
        match (source_ty, target_ty) {
            // int → int
            (s, t) if s.is_integer() && t.is_integer() => {
                let src_bits = self.int_bit_width(s);
                let tgt_bits = self.int_bit_width(t);
                let int_val = val.into_int_value();
                let tgt_int_ty = target_llvm.into_int_type();
                if tgt_bits > src_bits {
                    if s.is_signed() {
                        Ok(self
                            .builder
                            .build_int_s_extend(int_val, tgt_int_ty, "sext")
                            .unwrap()
                            .into())
                    } else {
                        Ok(self
                            .builder
                            .build_int_z_extend(int_val, tgt_int_ty, "zext")
                            .unwrap()
                            .into())
                    }
                } else if tgt_bits < src_bits {
                    Ok(self
                        .builder
                        .build_int_truncate(int_val, tgt_int_ty, "trunc")
                        .unwrap()
                        .into())
                } else {
                    Ok(val) // same width
                }
            }
            // int → float
            (s, _t) if s.is_integer() && target_ty.is_float() => {
                let int_val = val.into_int_value();
                let float_ty = target_llvm.into_float_type();
                if s.is_signed() {
                    Ok(self
                        .builder
                        .build_signed_int_to_float(int_val, float_ty, "sitofp")
                        .unwrap()
                        .into())
                } else {
                    Ok(self
                        .builder
                        .build_unsigned_int_to_float(int_val, float_ty, "uitofp")
                        .unwrap()
                        .into())
                }
            }
            // float → int
            (_s, t) if source_ty.is_float() && t.is_integer() => {
                let float_val = val.into_float_value();
                let int_ty = target_llvm.into_int_type();
                if t.is_signed() {
                    Ok(self
                        .builder
                        .build_float_to_signed_int(float_val, int_ty, "fptosi")
                        .unwrap()
                        .into())
                } else {
                    Ok(self
                        .builder
                        .build_float_to_unsigned_int(float_val, int_ty, "fptoui")
                        .unwrap()
                        .into())
                }
            }
            // float → float
            (_s, _t) if source_ty.is_float() && target_ty.is_float() => {
                let float_val = val.into_float_value();
                let float_ty = target_llvm.into_float_type();
                if matches!(target_ty, NyType::F64) {
                    Ok(self
                        .builder
                        .build_float_ext(float_val, float_ty, "fpext")
                        .unwrap()
                        .into())
                } else {
                    Ok(self
                        .builder
                        .build_float_trunc(float_val, float_ty, "fptrunc")
                        .unwrap()
                        .into())
                }
            }
            // bool → int
            (NyType::Bool, t) if t.is_integer() => {
                let bool_val = val.into_int_value();
                let int_ty = target_llvm.into_int_type();
                Ok(self
                    .builder
                    .build_int_z_extend(bool_val, int_ty, "boolext")
                    .unwrap()
                    .into())
            }
            _ => Err(vec![CompileError::type_error(
                format!("unsupported cast from {} to {}", source_ty, target_ty),
                Span::empty(0),
            )]),
        }
    }

    pub(super) fn int_bit_width(&self, ty: &NyType) -> u32 {
        match ty {
            NyType::I8 | NyType::U8 => 8,
            NyType::I16 | NyType::U16 => 16,
            NyType::I32 | NyType::U32 => 32,
            NyType::I64 | NyType::U64 => 64,
            NyType::I128 | NyType::U128 => 128,
            NyType::Bool => 1,
            _ => 32,
        }
    }
}
