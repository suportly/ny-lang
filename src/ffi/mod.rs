// src/ffi/mod.rs

pub mod cuda;
pub mod opencl;

use crate::common::CompileError;

pub trait FFI {
    fn name(&self) -> &str;
    fn generate_bindings(&self) -> Result<String, CompileError>;
}

pub fn get_ffi_backend(name: &str) -> Option<Box<dyn FFI>> {
    match name {
        "cuda" => Some(Box::new(cuda::CudaFFI)),
        "opencl" => Some(Box::new(opencl::OpenClFFI)),
        _ => None,
    }
}
