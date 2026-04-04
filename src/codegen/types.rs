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
            // Use the LARGEST type at each payload position (union-like layout)
            let has_payload = variants.iter().any(|(_, p)| !p.is_empty());
            if has_payload {
                let max_fields = variants.iter().map(|(_, p)| p.len()).max().unwrap_or(0);
                let mut field_types: Vec<BasicTypeEnum> = vec![context.i32_type().into()];
                for i in 0..max_fields {
                    let candidates: Vec<&NyType> =
                        variants.iter().filter_map(|(_, p)| p.get(i)).collect();
                    if candidates.is_empty() {
                        field_types.push(context.i32_type().into());
                    } else {
                        field_types.push(largest_llvm_type(context, &candidates));
                    }
                }
                context.struct_type(&field_types, false).into()
            } else {
                context.i32_type().into()
            }
        }
        NyType::Simd { elem, lanes } => {
            let elem_ty = ny_to_llvm(context, elem);
            match elem_ty {
                BasicTypeEnum::FloatType(ft) => ft.vec_type(*lanes).into(),
                BasicTypeEnum::IntType(it) => it.vec_type(*lanes).into(),
                _ => panic!("SIMD element must be scalar type"),
            }
        }
        NyType::Vec(_) => {
            // Vec is { ptr, len: i64, cap: i64, elem_size: i64 }
            context
                .struct_type(
                    &[
                        context.ptr_type(AddressSpace::default()).into(),
                        context.i64_type().into(),
                        context.i64_type().into(),
                        context.i64_type().into(),
                    ],
                    false,
                )
                .into()
        }
        NyType::HashMap(_, _) | NyType::Future(_) | NyType::Chan(_) => {
            // Opaque pointer to C runtime struct
            context.ptr_type(AddressSpace::default()).into()
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
        NyType::Optional(inner) => {
            // For pointer types: just a nullable pointer
            if inner.is_pointer() {
                context.ptr_type(AddressSpace::default()).into()
            } else {
                // For value types: { bool, T }
                let inner_llvm = ny_to_llvm(context, inner);
                context
                    .struct_type(&[context.bool_type().into(), inner_llvm], false)
                    .into()
            }
        }
        NyType::DynTrait(_) => {
            // Fat pointer: { data_ptr: *u8, vtable_ptr: *u8 }
            context
                .struct_type(
                    &[
                        context.ptr_type(AddressSpace::default()).into(),
                        context.ptr_type(AddressSpace::default()).into(),
                    ],
                    false,
                )
                .into()
        }
        NyType::Unit => {
            panic!("cannot convert {} to LLVM basic type", ty)
        }
    }
}

/// Pick the LLVM type with the largest store size among candidates.
pub fn largest_llvm_type<'ctx>(
    context: &'ctx Context,
    candidates: &[&NyType],
) -> BasicTypeEnum<'ctx> {
    let mut best_ty = ny_to_llvm(context, candidates[0]);
    let mut best_size = llvm_type_size(context, candidates[0]);
    for &ty in &candidates[1..] {
        let sz = llvm_type_size(context, ty);
        if sz > best_size {
            best_ty = ny_to_llvm(context, ty);
            best_size = sz;
        }
    }
    best_ty
}

/// Approximate byte size of an NyType (for enum union layout decisions).
fn llvm_type_size(_context: &Context, ty: &NyType) -> u64 {
    match ty {
        NyType::I8 | NyType::U8 | NyType::Bool => 1,
        NyType::I16 | NyType::U16 => 2,
        NyType::I32 | NyType::U32 | NyType::F32 => 4,
        NyType::I64 | NyType::U64 | NyType::F64 => 8,
        NyType::I128 | NyType::U128 => 16,
        NyType::Str | NyType::Slice(_) => 16, // {ptr, i64}
        NyType::Pointer(_) => 8,
        NyType::Struct { fields, .. } => fields
            .iter()
            .map(|(_, t)| llvm_type_size(_context, t))
            .sum(),
        NyType::Tuple(elems) => elems.iter().map(|t| llvm_type_size(_context, t)).sum(),
        NyType::Vec(_) => 32, // {ptr, i64, i64, i64}
        _ => 8,               // default pointer-sized
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
