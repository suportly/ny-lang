//! WebAssembly backend for the Ny compiler.
//!
//! This module handles the compilation of Ny code to WebAssembly (WASM).
//! It uses LLVM to generate a `.o` object file and then links it into a
//! final `.wasm` file using `wasm-ld`.

use inkwell::module::Module;
use inkwell::targets::{
    CodeModel, FileType, InitializationConfig, RelocMode, Target, TargetMachine, TargetTriple,
};
use inkwell::OptimizationLevel;
use std::path::Path;
use std::process::Command;

use crate::common::{CompileError, Span};

/// Emits a `.wasm` file from an LLVM module.
pub fn emit(
    module: &Module,
    output_path: &Path,
    opt_level: u8,
) -> Result<(), Vec<CompileError>> {
    // 1. Initialize the WebAssembly target in LLVM
    Target::initialize_webassembly(&InitializationConfig::default());

    // 2. Configure the target triple and create the target machine
    let triple = TargetTriple::create("wasm32-unknown-unknown");
    let target = Target::from_triple(&triple).map_err(|e| {
        vec![CompileError::new(
            format!("WASM target not available: {}", e),
            Span::empty(0),
        )]
    })?;

    let llvm_opt = match opt_level {
        0 => OptimizationLevel::None,
        1 => OptimizationLevel::Less,
        2 => OptimizationLevel::Default,
        _ => OptimizationLevel::Aggressive,
    };

    let target_machine = target
        .create_target_machine(
            &triple,
            "generic",
            "",
            llvm_opt,
            RelocMode::Pic, // Position-independent code
            CodeModel::Default,
        )
        .ok_or_else(|| {
            vec![CompileError::new(
                "Failed to create WASM target machine".to_string(),
                Span::empty(0),
            )]
        })?;

    // 3. Set the module's target triple
    module.set_triple(&triple);
    module.set_data_layout(&target_machine.get_target_data().get_data_layout());

    // 4. Emit the object file (`.o`)
    let obj_path = output_path.with_extension("o");
    target_machine
        .write_to_file(module, FileType::Object, &obj_path)
        .map_err(|e| {
            vec![CompileError::new(
                format!("Failed to emit WASM object file: {}", e),
                Span::empty(0),
            )]
        })?;

    // 5. Link the object file into a .wasm module using wasm-ld
    link_wasm_module(&obj_path, output_path)?;

    // 6. Clean up the temporary object file
    let _ = std::fs::remove_file(&obj_path);

    Ok(())
}

/// Links a WebAssembly object file (`.o`) into a final `.wasm` file.
fn link_wasm_module(obj_path: &Path, output_path: &Path) -> Result<(), Vec<CompileError>> {
    // Check if wasm-ld is available in the user's PATH
    let wasm_ld_path = match which::which("wasm-ld") {
        Ok(path) => path,
        Err(_) => {
            return Err(vec![CompileError::new(
                "Could not find `wasm-ld` in your PATH. Please install the `lld` package.".to_string(),
                Span::empty(0),
            )]);
        }
    };

    let status = Command::new(wasm_ld_path)
        .arg(obj_path)
        .arg("-o")
        .arg(output_path)
        .arg("--no-entry")      // Don't require a `_start` function
        .arg("--export-all")   // Export all functions
        .arg("--allow-undefined") // Allow undefined symbols (for host imports)
        .status();

    match status {
        Ok(s) if s.success() => Ok(()),
        Ok(s) => Err(vec![CompileError::new(
            format!("`wasm-ld` failed with exit code: {:?}", s.code()),
            Span::empty(0),
        )]),
        Err(e) => Err(vec![CompileError::new(
            format!("Failed to execute `wasm-ld`: {}", e),
            Span::empty(0),
        )]),
    }
}
