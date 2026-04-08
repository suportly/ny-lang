// src/ffi/opencl.rs

use crate::codegen::CodeGen;
use crate::common::CompileError;
use inkwell::types::{IntType, PointerType, VoidType};
use inkwell::AddressSpace;

/// Declares essential OpenCL API functions.
pub fn declare_opencl_functions(codegen: &mut CodeGen) -> Result<(), CompileError> {
    let context = codegen.context;
    let module = &codegen.module;

    let i32_type = context.i32_type();
    let i64_type = context.i64_type();
    let ptr_type = context.i8_type().ptr_type(AddressSpace::Generic);
    let void_ptr_type = context.void_type().ptr_type(AddressSpace::Generic);

    // clGetPlatformIDs
    let cl_get_platform_ids_type = i32_type.fn_type(
        &[
            i32_type.into(), // cl_uint num_entries
            ptr_type.into(), // cl_platform_id* platforms
            ptr_type.into(), // cl_uint* num_platforms
        ],
        false,
    );
    module.add_function("clGetPlatformIDs", cl_get_platform_ids_type, None);

    // clGetDeviceIDs
    let cl_get_device_ids_type = i32_type.fn_type(
        &[
            ptr_type.into(), // cl_platform_id platform
            i64_type.into(), // cl_device_type device_type
            i32_type.into(), // cl_uint num_entries
            ptr_type.into(), // cl_device_id* devices
            ptr_type.into(), // cl_uint* num_devices
        ],
        false,
    );
    module.add_function("clGetDeviceIDs", cl_get_device_ids_type, None);

    // clCreateContext
    let cl_create_context_type = ptr_type.fn_type(
        &[
            ptr_type.into(),    // const cl_context_properties* properties
            i32_type.into(),    // cl_uint num_devices
            ptr_type.into(),    // const cl_device_id* devices
            void_ptr_type.into(), // void (CL_CALLBACK* pfn_notify)(...)
            ptr_type.into(),    // void* user_data
            ptr_type.into(),    // cl_int* errcode_ret
        ],
        false,
    );
    module.add_function("clCreateContext", cl_create_context_type, None);

    // clCreateCommandQueue
    // Note: This is a deprecated function in newer OpenCL versions,
    // but it's simpler for an example.
    let cl_create_command_queue_type = ptr_type.fn_type(
        &[
            ptr_type.into(), // cl_context context
            ptr_type.into(), // cl_device_id device
            i64_type.into(), // cl_command_queue_properties properties
            ptr_type.into(), // cl_int* errcode_ret
        ],
        false,
    );
    module.add_function(
        "clCreateCommandQueue",
        cl_create_command_queue_type,
        None,
    );

    Ok(())
}
