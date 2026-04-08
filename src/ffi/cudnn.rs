// src/ffi/cudnn.rs

use crate::codegen::CodeGen;
use crate::common::CompileError;
use inkwell::types::{IntType, PointerType, VoidType};
use inkwell::AddressSpace;

/// Declares essential cuDNN API functions.
pub fn declare_cudnn_functions(codegen: &mut CodeGen) -> Result<(), CompileError> {
    let context = codegen.context;
    let module = &codegen.module;

    let i32_type = context.i32_type();
    let ptr_type = context.i8_type().ptr_type(AddressSpace::Generic);

    // cudnnCreate
    let cudnn_create_type = i32_type.fn_type(&[ptr_type.into()], false);
    module.add_function("cudnnCreate", cudnn_create_type, None);

    // cudnnDestroy
    let cudnn_destroy_type = i32_type.fn_type(&[ptr_type.into()], false);
    module.add_function("cudnnDestroy", cudnn_destroy_type, None);

    // cudnnCreateTensorDescriptor
    let cudnn_create_tensor_desc_type = i32_type.fn_type(&[ptr_type.into()], false);
    module.add_function(
        "cudnnCreateTensorDescriptor",
        cudnn_create_tensor_desc_type,
        None,
    );

    // cudnnDestroyTensorDescriptor
    let cudnn_destroy_tensor_desc_type = i32_type.fn_type(&[ptr_type.into()], false);
    module.add_function(
        "cudnnDestroyTensorDescriptor",
        cudnn_destroy_tensor_desc_type,
        None,
    );

    Ok(())
}
