// src/ffi/mod.rs

pub mod bindings;
pub mod cuda;
pub mod cudnn;
pub mod opencl;

use crate::common::{CompileError, NyType, Span};
use crate::parser::ast::Program;
use inkwell::module::Module;

/// Manages the Foreign Function Interface for the Ny Lang compiler.
pub struct FFI<'a, 'ctx> {
    // We might need context, module, etc. from codegen
    codegen: &'a mut crate::codegen::CodeGen<'ctx>,
}

impl<'a, 'ctx> FFI<'a, 'ctx> {
    /// Creates a new FFI manager.
    pub fn new(codegen: &'a mut crate::codegen::CodeGen<'ctx>) -> Self {
        FFI { codegen }
    }

    /// Processes the FFI-related parts of the AST.
    /// This could involve looking for `extern "C" {}` blocks
    /// or specific annotations for GPU libraries.
    pub fn process_program(&mut self, program: &Program) -> Result<(), Vec<CompileError>> {
        // Here we would iterate through the program to find FFI blocks.
        // For now, this is a placeholder.
        Ok(())
    }

    /// Generates LLVM IR for the FFI functions.
    pub fn codegen_ffi_declarations(&mut self) -> Result<(), CompileError> {
        // This is where we would declare the external functions in LLVM.
        // For example, calling into a function that handles CUDA bindings.
        cuda::declare_cuda_functions(self.codegen)?;
        cudnn::declare_cudnn_functions(self.codegen)?;
        opencl::declare_opencl_functions(self.codegen)?;
        Ok(())
    }
}
