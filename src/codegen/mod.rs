pub mod builtins;
mod cast;
mod expr;
mod inference;
mod ops;
mod print;
mod runtime_decls;
mod stmt;
pub mod types;

use std::collections::HashMap;
use std::path::Path;
use std::process::Command;

use inkwell::basic_block::BasicBlock;
use inkwell::builder::Builder;
use inkwell::context::Context;
use inkwell::module::Module;
use inkwell::passes::PassBuilderOptions;
use inkwell::targets::{
    CodeModel, FileType, InitializationConfig, RelocMode, Target, TargetMachine, TargetTriple,
};
use inkwell::types::{BasicType, BasicTypeEnum};
use inkwell::values::{FunctionValue, PointerValue};
use inkwell::OptimizationLevel;

use crate::common::{CompileError, NyType, Span};
use crate::ffi; // Import the new FFI module
use crate::parser::ast::*;
use types::ny_to_llvm;

pub fn generate(
    program: &Program,
    source_path: &Path,
    output_path: &Path,
    opt_level: u8,
    emit: &str,
    target: &str,
    extra_libs: &[String],
) -> Result<(), Vec<CompileError>> {
    let context = Context::create();

    let module = context.create_module(source_path.to_str().unwrap_or("main"));
    let builder = context.create_builder();

    let mut codegen = CodeGen {
        context: &context,
        module,
        builder,
        variables: HashMap::new(),
        functions: HashMap::new(),
        struct_types: HashMap::new(),
        enum_variants: HashMap::new(),
        loop_stack: Vec::new(),
        defer_stack: Vec::new(),
        closure_captures: HashMap::new(),
        opt_level,
        trait_defs: HashMap::new(),
        vtables: HashMap::new(),
        trait_impls: HashMap::new(),
        type_aliases: HashMap::new(),
    };

    codegen.compile_program(program)?;

    let module = codegen.module;

    // Run optimization passes
    if opt_level > 0 {
        let pass_options = PassBuilderOptions::create();
        let target_machine = create_target_machine(opt_level);

        let passes = match opt_level {
            1 => "default<O1>",
            2 => "default<O2>",
            _ => "default<O3>",
        };

        module
            .run_passes(passes, &target_machine, pass_options)
            .map_err(|e| {
                vec![CompileError::syntax(
                    format!("LLVM optimization failed: {}", e.to_string()),
                    Span::empty(0),
                )]
            })?;
    }

    if target == "wasm32" {
        return emit_wasm(&module, output_path, opt_level);
    }

    match emit {
        "llvm-ir" => {
            print!("{}", module.print_to_string().to_string());
            Ok(())
        }
        "obj" => {
            let obj_path = output_path.with_extension("o");
            emit_object_file(&module, &obj_path, opt_level)?;
            Ok(())
        }
        _ => {
            let obj_path = output_path.with_extension("o");
            emit_object_file(&module, &obj_path, opt_level)?;
            // Add GPU libraries to the linker
            let mut final_libs = extra_libs.to_vec();
            final_libs.extend(vec![
                "cuda".to_string(),
                "cudnn".to_string(),
                "OpenCL".to_string(),
            ]);
            link_executable(&obj_path, output_path, &final_libs)?;
            let _ = std::fs::remove_file(&obj_path);
            Ok(())
        }
    }
}

fn emit_wasm(module: &Module, output_path: &Path, opt_level: u8) -> Result<(), Vec<CompileError>> {
    // Initialize WASM target
    Target::initialize_webassembly(&InitializationConfig::default());

    let triple = TargetTriple::create("wasm32-unknown-unknown");
    let target = Target::from_triple(&triple).map_err(|e| {
        vec![CompileError::syntax(
            format!("wasm32 target not available: {}", e.to_string()),
            Span::empty(0),
        )]
    })?;

    let llvm_opt = match opt_level {
        0 => OptimizationLevel::None,
        1 => OptimizationLevel::Less,
        2 => OptimizationLevel::Default,
        _ => OptimizationLevel::Aggressive,
    };

    let machine = target
        .create_target_machine(
            &triple,
            "generic",
            "",
            llvm_opt,
            RelocMode::Default,
            CodeModel::Default,
        )
        .ok_or_else(|| {
            vec![CompileError::syntax(
                "failed to create wasm32 target machine".to_string(),
                Span::empty(0),
            )]
        })?;

    // Set the module's target triple
    module.set_triple(&triple);

    // Emit object file (.o)
    let obj_path = output_path.with_extension("o");
    machine
        .write_to_file(module, FileType::Object, &obj_path)
        .map_err(|e| {
            vec![CompileError::syntax(
                format!("failed to emit wasm object: {}", e.to_string()),
                Span::empty(0),
            )]
        })?;

    // Link with wasm-ld to produce .wasm
    let status = Command::new("wasm-ld")
        .arg(&obj_path)
        .arg("-o")
        .arg(output_path)
        .arg("--no-entry")
        .arg("--export-all")
        .arg("--allow-undefined")
        .status();

    match status {
        Ok(s) if s.success() => {
            let _ = std::fs::remove_file(&obj_path);
            Ok(())
        }
        _ => {
            // wasm-ld not available — keep the .o file
            eprintln!(
                "warning: wasm-ld not found. Keeping object file: {}",
                obj_path.display()
            );
            Ok(())
        }
    }
}

pub struct CodeGen<'ctx> {
    pub context: &'ctx Context,
    pub module: Module<'ctx>,
    pub builder: Builder<'ctx>,
    pub variables: HashMap<String, (PointerValue<'ctx>, Option<NyType>)>,
    pub functions: HashMap<String, FunctionValue<'ctx>>,
    pub struct_types: HashMap<String, (inkwell::types::StructType<'ctx>, Struct)>,
    pub enum_variants: HashMap<String, (u32, Vec<NyType>)>,
    pub loop_stack: Vec<(BasicBlock<'ctx>, BasicBlock<'ctx>)>,
    pub defer_stack: Vec<Vec<Expr>>,
    pub closure_captures: HashMap<String, Vec<(String, NyType, PointerValue<'ctx>)>>,
    pub opt_level: u8,
    pub trait_defs: HashMap<String, Trait>,
    pub vtables: HashMap<(String, String), (PointerValue<'ctx>, HashMap<String, u32>)>, // (impl_type, trait_name) -> vtable
    pub trait_impls: HashMap<String, HashMap<String, Impl>>,
    pub type_aliases: HashMap<String, NyType>,
}

impl<'ctx> CodeGen<'ctx> {
    fn compile_program(&mut self, program: &Program) -> Result<(), Vec<CompileError>> {
        runtime_decls::declare_runtime_functions(self);
        builtins::declare_builtin_functions(self);

        // Declare FFI functions for GPU libraries
        self.compile_ffi_declarations().map_err(|e| vec![e])?;

        for item in &program.items {
            if let Item::Function(f) = item {
                self.declare_function(f);
            }
        }

        for item in &program.items {
            match item {
                Item::Function(f) => self.compile_function(f).map_err(|e| vec![e])?,
                Item::Struct(s) => self.declare_struct(s),
                Item::Enum(e) => self.declare_enum(e),
                Item::Impl(i) => self.compile_impl(i),
                Item::Trait(t) => {
                    self.trait_defs.insert(t.name.clone(), t.clone());
                }
                Item::TypeAlias(name, ty) => {
                    self.type_aliases.insert(name.clone(), ty.clone());
                }
                Item::Use { .. } => {}
            }
        }
        Ok(())
    }

    fn compile_ffi_declarations(&mut self) -> Result<(), CompileError> {
        ffi::cuda::declare_cuda_functions(self)?;
        ffi::cudnn::declare_cudnn_functions(self)?;
        ffi::opencl::declare_opencl_functions(self)?;
        Ok(())
    }

    fn get_variable(&self, name: &str) -> Option<&(PointerValue<'ctx>, Option<NyType>)> {
        self.variables.get(name)
    }

    fn get_function(&self, name: &str) -> Option<&FunctionValue<'ctx>> {
        self.functions.get(name)
    }

    fn enter_scope(&mut self) -> HashMap<String, (PointerValue<'ctx>, Option<NyType>)> {
        self.variables.clone()
    }

    fn exit_scope(&mut self, scope: HashMap<String, (PointerValue<'ctx>, Option<NyType>)>) {
        self.variables = scope;
    }

    fn create_entry_block_alloca<T: BasicType<'ctx>>(
        &self,
        func: FunctionValue,
        name: &str,
    ) -> PointerValue<'ctx> {
        let builder = self.context.create_builder();
        let entry = func.get_first_basic_block().unwrap();

        match entry.get_first_instruction() {
            Some(first_instr) => builder.position_before(&first_instr),
            None => builder.position_at_end(entry),
        }

        builder.build_alloca(T::get_type(self.context), name)
    }
}

fn create_target_machine(opt_level: u8) -> TargetMachine {
    Target::initialize_native(&InitializationConfig::default()).unwrap();
    let triple = TargetTriple::create(TargetMachine::get_default_triple().to_str().unwrap());
    let target = Target::from_triple(&triple).unwrap();
    let cpu = TargetMachine::get_host_cpu_name().to_string();
    let features = TargetMachine::get_host_cpu_features().to_string();
    let opt = match opt_level {
        0 => OptimizationLevel::None,
        1 => OptimizationLevel::Less,
        2 => OptimizationLevel::Default,
        _ => OptimizationLevel::Aggressive,
    };
    target
        .create_target_machine(&triple, &cpu, &features, opt, RelocMode::PIC, CodeModel::Default)
        .unwrap()
}

fn emit_object_file(
    module: &Module,
    path: &Path,
    opt_level: u8,
) -> Result<(), Vec<CompileError>> {
    let target_machine = create_target_machine(opt_level);
    target_machine
        .write_to_file(module, FileType::Object, path)
        .map_err(|e| {
            vec![CompileError::syntax(
                format!("failed to emit object file: {}", e.to_string()),
                Span::empty(0),
            )]
        })
}

fn link_executable(
    obj_path: &Path,
    output_path: &Path,
    extra_libs: &[String],
) -> Result<(), Vec<CompileError>> {
    let mut command = Command::new("cc");
    let command = command
        .arg(obj_path)
        .arg("-o")
        .arg(output_path)
        .arg("-lm"); // link math library

    for lib in extra_libs {
        command.arg(format!("-l{}", lib));
    }

    let output = command.output().map_err(|e| {
        vec![CompileError::syntax(
            format!("linking failed: {}", e.to_string()),
            Span::empty(0),
        )]
    })?;

    if !output.status.success() {
        return Err(vec![CompileError::syntax(
            format!(
                "linker error: {}",
                String::from_utf8_lossy(&output.stderr)
            ),
            Span::empty(0),
        )]);
    }

    Ok(())
}
