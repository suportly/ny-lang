use inkwell::context::Context;
use inkwell::types::{BasicType, BasicTypeEnum, StructType};
use inkwell::AddressSpace;

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
        NyType::Str => str_type(context).into(),
        NyType::Array { elem, size } => {
            let elem_llvm = ny_to_llvm(context, elem);
            elem_llvm.array_type(*size as u32).into()
        }
        NyType::Pointer(_) => context.ptr_type(AddressSpace::default()).into(),
        NyType::Struct { name, fields } => {
            let field_types: Vec<BasicTypeEnum> =
                fields.iter().map(|(_, t)| ny_to_llvm(context, t)).collect();
            let struct_ty = context.opaque_struct_type(name);
            struct_ty.set_body(&field_types, false);
            struct_ty.into()
        }
        NyType::Enum { variants, .. } => {
            // If any variant has a payload, use struct type { i32, payload... }
            let has_payload = variants.iter().any(|(_, p)| !p.is_empty());
            if has_payload {
                let max_fields = variants.iter().map(|(_, p)| p.len()).max().unwrap_or(0);
                let mut field_types: Vec<BasicTypeEnum> = vec![context.i32_type().into()];
                for i in 0..max_fields {
                    let mut found = false;
                    for (_, payload) in variants {
                        if let Some(ty) = payload.get(i) {
                            field_types.push(ny_to_llvm(context, ty));
                            found = true;
                            break;
                        }
                    }
                    if !found {
                        field_types.push(context.i32_type().into());
                    }
                }
                context.struct_type(&field_types, false).into()
            } else {
                context.i32_type().into()
            }
        }
        NyType::Vec(_) => {
            // Vec is { ptr, len: i64, cap: i64 }
            context
                .struct_type(
                    &[
                        context.ptr_type(AddressSpace::default()).into(),
                        context.i64_type().into(),
                        context.i64_type().into(),
                    ],
                    false,
                )
                .into()
        }
        NyType::Slice(_) => {
            // Slice is { ptr, len } like str
            context
                .struct_type(
                    &[
                        context.ptr_type(AddressSpace::default()).into(),
                        context.i64_type().into(),
                    ],
                    false,
                )
                .into()
        }
        NyType::Tuple(elems) => {
            let field_types: Vec<BasicTypeEnum> =
                elems.iter().map(|t| ny_to_llvm(context, t)).collect();
            context.struct_type(&field_types, false).into()
        }
        NyType::Function { .. } => {
            // Function pointers are opaque pointers in LLVM
            context.ptr_type(AddressSpace::default()).into()
        }
        NyType::Unit => {
            panic!("cannot convert {} to LLVM basic type", ty)
        }
    }
}

/// The str type is { ptr, i64 } — pointer to bytes + byte length
pub fn str_type<'ctx>(context: &'ctx Context) -> StructType<'ctx> {
    context.struct_type(
        &[
            context.ptr_type(AddressSpace::default()).into(),
            context.i64_type().into(),
        ],
        false,
    )
}
