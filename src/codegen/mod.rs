pub mod builtins;
mod cast;
mod expr;
mod inference;
mod ops;
mod print;
mod runtime_decls;
mod stmt;
pub mod types;
mod wasm;

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
                vec![CompileError::new(
                    format!("LLVM optimization failed: {}", e.to_string()),
                    Span::empty(0),
                )]
            })?;
    }

    if target == "wasm32" {
        return wasm::emit(&module, output_path, opt_level);
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
            link_executable(&obj_path, output_path, extra_libs)?;
            let _ = std::fs::remove_file(&obj_path);
            Ok(())
        }
    }
}

fn create_target_machine(opt_level: u8) -> TargetMachine {
    Target::initialize_native(&InitializationConfig::default()).unwrap();

    let triple = TargetTriple::create(std::env::consts::TRIPLE);
    let target = Target::from_triple(&triple).unwrap();

    let llvm_opt = match opt_level {
        0 => OptimizationLevel::None,
        1 => OptimizationLevel::Less,
        2 => OptimizationLevel::Default,
        _ => OptimizationLevel::Aggressive,
    };

    target
        .create_target_machine(
            &triple,
            "generic",
            "",
            llvm_opt,
            RelocMode::Default,
            CodeModel::Default,
        )
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
            vec![CompileError::new(
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
    command.arg(obj_path);
    command.arg("-o");
    command.arg(output_path);

    for lib in extra_libs {
        command.arg(format!("-l{}", lib));
    }

    let status = command.status().map_err(|e| {
        vec![CompileError::new(
            format!("failed to execute linker: {}", e.to_string()),
            Span::empty(0),
        )]
    })?;

    if !status.success() {
        return Err(vec![CompileError::new(
            "linker failed".to_string(),
            Span::empty(0),
        )]);
    }

    Ok(())
}

pub struct LoopFrame<'a> {
    loop_start: BasicBlock<'a>,
    loop_end: BasicBlock<'a>,
}

pub struct DeferFrame<'a> {
    pub expr: &'a Expr,
    pub parent_fn: FunctionValue<'a>,
}

pub struct CodeGen<'ctx> {
    pub context: &'ctx Context,
    pub module: Module<'ctx>,
    pub builder: Builder<'ctx>,
    pub variables: HashMap<String, (PointerValue<'ctx>, NyType)>,
    pub functions: HashMap<String, FunctionValue<'ctx>>,
    pub struct_types: HashMap<String, BasicTypeEnum<'ctx>>,
    pub enum_variants: HashMap<String, (u32, Vec<NyType>)>,
    pub loop_stack: Vec<LoopFrame<'ctx>>,
    pub defer_stack: Vec<Vec<DeferFrame<'ctx>>>,
    pub closure_captures: HashMap<String, Vec<(String, NyType)>>,
    pub opt_level: u8,
    pub trait_defs: HashMap<String, TraitDef>,
    pub vtables: HashMap<String, PointerValue<'ctx>>,
    pub trait_impls: HashMap<String, HashMap<String, String>>,
    pub type_aliases: HashMap<String, NyType>,
}

impl<'ctx> CodeGen<'ctx> {
    fn compile_program(&mut self, program: &Program) -> Result<(), Vec<CompileError>> {
        let mut errors = Vec::new();

        // Pass 1: Register all type definitions (structs, enums, traits, aliases)
        for stmt in &program.statements {
            match stmt {
                Statement::StructDef(s, _) => {
                    let struct_type = self.context.opaque_struct_type(&s.name);
                    self.struct_types
                        .insert(s.name.clone(), struct_type.into());
                }
                Statement::EnumDef(e, _) => {
                    for (i, variant) in e.variants.iter().enumerate() {
                        let variant_types = variant.types.clone().unwrap_or_default();
                        self.enum_variants
                            .insert(variant.name.clone(), (i as u32, variant_types));
                    }
                }
                Statement::TraitDef(t, _) => {
                    self.trait_defs.insert(t.name.clone(), t.clone());
                }
                Statement::TypeAlias(name, ty, _) => {
                    self.type_aliases.insert(name.clone(), ty.clone());
                }
                _ => (),
            }
        }

        // Pass 2: Define struct bodies
        for stmt in &program.statements {
            if let Statement::StructDef(s, _) = stmt {
                if let Some(BasicTypeEnum::StructType(struct_type)) =
                    self.struct_types.get(&s.name)
                {
                    let field_types: Vec<BasicTypeEnum> = s
                        .fields
                        .iter()
                        .map(|f| ny_to_llvm(self.context, &f.ny_type, &self.struct_types))
                        .collect();
                    struct_type.set_body(&field_types, false);
                }
            }
        }

        // Pass 3: Compile function declarations (prototypes)
        for stmt in &program.statements {
            if let Statement::FunctionDef(f, _) = stmt {
                if let Err(e) = self.compile_function_decl(f) {
                    errors.push(e);
                }
            }
        }

        // Pass 4: Compile trait implementations
        for stmt in &program.statements {
            if let Statement::TraitImpl(t, _) = stmt {
                if let Err(mut e) = self.compile_trait_impl(t) {
                    errors.append(&mut e);
                }
            }
        }

        // Pass 5: Compile function bodies
        for stmt in &program.statements {
            if let Statement::FunctionDef(f, _) = stmt {
                if let Err(e) = self.compile_function_body(f) {
                    errors.push(e);
                }
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}
