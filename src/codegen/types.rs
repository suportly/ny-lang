use inkwell::context::Context;
use inkwell::types::BasicTypeEnum;

use crate::common::NyType;

pub fn ny_to_llvm<'ctx>(context: &'ctx Context, ty: &NyType) -> BasicTypeEnum<'ctx> {
    match ty {
        NyType::I8 => context.i8_type().into(),
        NyType::I16 => context.i16_type().into(),
        NyType::I32 => context.i32_type().into(),
        NyType::I64 => context.i64_type().into(),
        NyType::I128 => context.i128_type().into(),
        NyType::U8 => context.i8_type().into(),
        NyType::U16 => context.i16_type().into(),
        NyType::U32 => context.i32_type().into(),
        NyType::U64 => context.i64_type().into(),
        NyType::U128 => context.i128_type().into(),
        NyType::F32 => context.f32_type().into(),
        NyType::F64 => context.f64_type().into(),
        NyType::Bool => context.bool_type().into(),
        NyType::Unit | NyType::Function { .. } => {
            panic!("cannot convert {} to LLVM basic type", ty)
        }
    }
}
