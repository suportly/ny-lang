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
    CodeModel, FileType, InitializationConfig, RelocMode, Target, TargetMachine,
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
            link_executable(&obj_path, output_path)?;
            let _ = std::fs::remove_file(&obj_path);
            Ok(())
        }
    }
}

fn create_target_machine(opt_level: u8) -> TargetMachine {
    Target::initialize_native(&InitializationConfig::default())
        .expect("failed to initialize native target");

    let triple = TargetMachine::get_default_triple();
    let target = Target::from_triple(&triple).expect("failed to get target from triple");

    let cpu = TargetMachine::get_host_cpu_name();
    let features = TargetMachine::get_host_cpu_features();

    let llvm_opt = match opt_level {
        0 => OptimizationLevel::None,
        1 => OptimizationLevel::Less,
        2 => OptimizationLevel::Default,
        _ => OptimizationLevel::Aggressive,
    };

    target
        .create_target_machine(
            &triple,
            cpu.to_str().unwrap_or("generic"),
            features.to_str().unwrap_or(""),
            llvm_opt,
            RelocMode::Default,
            CodeModel::Default,
        )
        .expect("failed to create target machine")
}

fn emit_object_file(module: &Module, path: &Path, opt_level: u8) -> Result<(), Vec<CompileError>> {
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

fn link_executable(obj_path: &Path, output_path: &Path) -> Result<(), Vec<CompileError>> {
    let mut cmd = Command::new("cc");
    cmd.arg(obj_path)
        .arg("-o")
        .arg(output_path)
        .arg("-no-pie")
        .arg("-lm")
        .arg("-lc")
        .arg("-lpthread");

    // Link all runtime C files (hashmap.c, arena.c, etc.)
    for rt_name in &["hashmap.c", "arena.c", "channel.c", "threadpool.c", "string.c", "json.c"] {
        if let Some(rt_path) = find_runtime_file(rt_name) {
            cmd.arg(rt_path);
        }
    }

    let status = cmd.status().map_err(|e| {
        vec![CompileError::syntax(
            format!("failed to invoke linker 'cc': {}", e),
            Span::empty(0),
        )]
    })?;

    if !status.success() {
        return Err(vec![CompileError::syntax(
            "linker 'cc' failed".to_string(),
            Span::empty(0),
        )]);
    }

    Ok(())
}

fn find_runtime_file(name: &str) -> Option<std::path::PathBuf> {
    // Try relative to current executable
    if let Ok(exe) = std::env::current_exe() {
        let dir = exe.parent()?;
        // Try: exe_dir/../runtime/name (development layout)
        let dev_path = dir.join("..").join("..").join("runtime").join(name);
        if dev_path.exists() {
            return Some(dev_path);
        }
        // Try: exe_dir/runtime/name
        let install_path = dir.join("runtime").join(name);
        if install_path.exists() {
            return Some(install_path);
        }
    }
    // Try current working directory
    let cwd_path = Path::new("runtime").join(name);
    if cwd_path.exists() {
        return Some(cwd_path);
    }
    None
}

// ---------------------------------------------------------------------------
// LoopFrame: tracks break/continue targets for nested loops
// ---------------------------------------------------------------------------

pub(super) struct LoopFrame<'ctx> {
    pub(super) break_bb: BasicBlock<'ctx>,
    pub(super) continue_bb: BasicBlock<'ctx>,
}

// ---------------------------------------------------------------------------
// CodeGen
// ---------------------------------------------------------------------------

pub(crate) struct CodeGen<'ctx> {
    pub(super) context: &'ctx Context,
    pub(super) module: Module<'ctx>,
    pub(super) builder: Builder<'ctx>,
    pub(super) variables: HashMap<String, (PointerValue<'ctx>, NyType)>,
    pub(super) functions: HashMap<String, (FunctionValue<'ctx>, Vec<NyType>, NyType)>,
    /// Struct name -> ordered list of (field_name, field_type)
    pub(super) struct_types: HashMap<String, Vec<(String, NyType)>>,
    /// Enum name -> ordered list of (variant_name, payload_types)
    pub(super) enum_variants: HashMap<String, Vec<(String, Vec<NyType>)>>,
    pub(super) loop_stack: Vec<LoopFrame<'ctx>>,
    /// Stack of deferred expressions per function scope
    pub(super) defer_stack: Vec<(Expr, FunctionValue<'ctx>)>,
    /// Closure captures: closure_var_name → (lambda_fn_name, capture_alloca_names)
    /// Each capture has a dedicated alloca "closure_{id}_cap_{name}" that holds
    /// the value at lambda creation time (capture-by-value semantics).
    pub(super) closure_captures: HashMap<String, (String, Vec<(String, NyType)>)>,
}

impl<'ctx> CodeGen<'ctx> {
    // ------------------------------------------------------------------
    // Type annotation resolution (mirrors resolver logic, but for codegen)
    // ------------------------------------------------------------------

    fn resolve_type_annotation(&self, annotation: &TypeAnnotation) -> NyType {
        match annotation {
            TypeAnnotation::Named { name, .. } => {
                if let Some(ty) = NyType::from_name(name) {
                    return ty;
                }
                if name == "()" {
                    return NyType::Unit;
                }
                // Check Vec<StructName> pattern
                if let Some(inner) = name.strip_prefix("Vec<").and_then(|s| s.strip_suffix('>')) {
                    if let Some(fields) = self.struct_types.get(inner) {
                        return NyType::Vec(Box::new(NyType::Struct {
                            name: inner.to_string(),
                            fields: fields.clone(),
                        }));
                    }
                    if let Some(variants) = self.enum_variants.get(inner) {
                        return NyType::Vec(Box::new(NyType::Enum {
                            name: inner.to_string(),
                            variants: variants.clone(),
                        }));
                    }
                }
                // Check registered struct types
                if let Some(fields) = self.struct_types.get(name) {
                    return NyType::Struct {
                        name: name.clone(),
                        fields: fields.clone(),
                    };
                }
                // Check registered enum types
                if let Some(variant_defs) = self.enum_variants.get(name) {
                    return NyType::Enum {
                        name: name.clone(),
                        variants: variant_defs.clone(),
                    };
                }
                // Fallback
                NyType::I32
            }
            TypeAnnotation::Array { elem, size, .. } => {
                let elem_ty = self.resolve_type_annotation(elem);
                NyType::Array {
                    elem: Box::new(elem_ty),
                    size: *size,
                }
            }
            TypeAnnotation::Pointer { inner, .. } => {
                let inner_ty = self.resolve_type_annotation(inner);
                NyType::Pointer(Box::new(inner_ty))
            }
            TypeAnnotation::Tuple { elements, .. } => {
                let elem_types: Vec<NyType> = elements
                    .iter()
                    .map(|e| self.resolve_type_annotation(e))
                    .collect();
                NyType::Tuple(elem_types)
            }
            TypeAnnotation::Slice { elem, .. } => {
                let elem_ty = self.resolve_type_annotation(elem);
                NyType::Slice(Box::new(elem_ty))
            }
            TypeAnnotation::Function { params, ret, .. } => {
                let param_types: Vec<NyType> = params
                    .iter()
                    .map(|p| self.resolve_type_annotation(p))
                    .collect();
                let ret_ty = self.resolve_type_annotation(ret);
                NyType::Function {
                    params: param_types,
                    ret: Box::new(ret_ty),
                }
            }
        }
    }

    // ------------------------------------------------------------------
    // LLVM struct type lookup/creation (named, so each name is unique)
    // ------------------------------------------------------------------

    fn get_or_create_llvm_struct_type(
        &self,
        name: &str,
        fields: &[(String, NyType)],
    ) -> inkwell::types::StructType<'ctx> {
        // Check if already registered in the LLVM module
        if let Some(st) = self.context.get_struct_type(name) {
            return st;
        }
        let field_llvm_types: Vec<BasicTypeEnum> = fields
            .iter()
            .map(|(_, t)| ny_to_llvm(self.context, t))
            .collect();
        let st = self.context.opaque_struct_type(name);
        st.set_body(&field_llvm_types, false);
        st
    }

    // ------------------------------------------------------------------
    // Check if an enum has any data-carrying variant
    // ------------------------------------------------------------------

    fn enum_has_payload(&self, enum_name: &str) -> bool {
        if let Some(variants) = self.enum_variants.get(enum_name) {
            variants.iter().any(|(_, payload)| !payload.is_empty())
        } else {
            false
        }
    }

    /// Build the LLVM struct type for a data-carrying enum: { i32, field0, field1, ... }
    /// where fields are the UNION of all payload positions across variants (largest at each position).
    pub(super) fn enum_struct_type(&self, enum_name: &str) -> inkwell::types::StructType<'ctx> {
        let variants = self.enum_variants.get(enum_name).unwrap_or_else(|| {
            panic!(
                "internal compiler error: enum '{}' not registered",
                enum_name
            )
        });
        // Find max payload count across all variants
        let max_fields = variants.iter().map(|(_, p)| p.len()).max().unwrap_or(0);
        let mut field_types: Vec<BasicTypeEnum<'ctx>> = vec![self.context.i32_type().into()]; // tag
        for i in 0..max_fields {
            // Find the largest type at position i across all variants
            // For simplicity, use the first variant that has this position
            let mut found = false;
            for (_, payload) in variants {
                if let Some(ty) = payload.get(i) {
                    field_types.push(ny_to_llvm(self.context, ty));
                    found = true;
                    break;
                }
            }
            if !found {
                field_types.push(self.context.i32_type().into()); // fallback
            }
        }
        self.context.struct_type(&field_types, false)
    }

    // ------------------------------------------------------------------
    // Compile program: three passes (struct types, function decls, bodies)
    // ------------------------------------------------------------------

    fn compile_program(&mut self, program: &Program) -> Result<(), Vec<CompileError>> {
        // Pass 0: Register all LLVM named struct types and enum variant lists
        for item in &program.items {
            if let Item::StructDef { name, fields, .. } = item {
                let resolved_fields: Vec<(String, NyType)> = fields
                    .iter()
                    .map(|(fname, fty)| {
                        let ny_ty = self.resolve_type_annotation(fty);
                        (fname.clone(), ny_ty)
                    })
                    .collect();
                self.get_or_create_llvm_struct_type(name, &resolved_fields);
                self.struct_types.insert(name.clone(), resolved_fields);
            }
            if let Item::EnumDef { name, variants, .. } = item {
                // Convert EnumVariantDef to (name, payload_types)
                let resolved: Vec<(String, Vec<NyType>)> = variants
                    .iter()
                    .map(|v| {
                        let payload_types: Vec<NyType> = v
                            .payload
                            .iter()
                            .map(|ty_ann| self.resolve_type_annotation(ty_ann))
                            .collect();
                        (v.name.clone(), payload_types)
                    })
                    .collect();
                self.enum_variants.insert(name.clone(), resolved);
            }
        }

        // Pass 0b: Register extern function declarations
        for item in &program.items {
            if let Item::ExternBlock { functions, .. } = item {
                for ext_fn in functions {
                    let ret_ty = self.resolve_type_annotation(&ext_fn.return_type);
                    let param_types: Vec<NyType> = ext_fn
                        .params
                        .iter()
                        .map(|p| self.resolve_type_annotation(&p.ty))
                        .collect();

                    let llvm_param_types: Vec<BasicTypeEnum> = param_types
                        .iter()
                        .map(|t| ny_to_llvm(self.context, t))
                        .collect();
                    let param_meta: Vec<_> = llvm_param_types.iter().map(|t| (*t).into()).collect();

                    let fn_type = match &ret_ty {
                        NyType::Unit => self
                            .context
                            .void_type()
                            .fn_type(&param_meta, ext_fn.variadic),
                        ty => ny_to_llvm(self.context, ty).fn_type(&param_meta, ext_fn.variadic),
                    };

                    let function = self.module.add_function(&ext_fn.name, fn_type, None);
                    self.functions
                        .insert(ext_fn.name.clone(), (function, param_types, ret_ty));
                }
            }
        }

        // Pass 0c: Flatten impl methods into top-level functions with qualified names
        let mut impl_methods: Vec<(String, &Vec<Param>, &TypeAnnotation, &Expr, Span)> = Vec::new();
        for item in &program.items {
            if let Item::ImplBlock {
                type_name,
                methods,
                trait_name: _,
                ..
            } = item
            {
                for method in methods {
                    if let Item::FunctionDef {
                        name,
                        params,
                        return_type,
                        body,
                        span,
                        type_params: _,
                    } = method
                    {
                        let qualified_name = format!("{}_{}", type_name, name);
                        impl_methods.push((qualified_name, params, return_type, body, *span));
                    }
                }
            }
        }

        // Pass 1: Declare all functions (forward references)
        for (qualified_name, params, return_type, _, _) in &impl_methods {
            let ret_ty = self.resolve_type_annotation(return_type);
            let param_types: Vec<NyType> = params
                .iter()
                .map(|p| self.resolve_type_annotation(&p.ty))
                .collect();

            let llvm_param_types: Vec<BasicTypeEnum> = param_types
                .iter()
                .map(|t| ny_to_llvm(self.context, t))
                .collect();

            let param_meta: Vec<_> = llvm_param_types.iter().map(|t| (*t).into()).collect();

            let fn_type = match &ret_ty {
                NyType::Unit => self.context.void_type().fn_type(&param_meta, false),
                ty => ny_to_llvm(self.context, ty).fn_type(&param_meta, false),
            };

            let function = self.module.add_function(qualified_name, fn_type, None);
            self.functions
                .insert(qualified_name.clone(), (function, param_types, ret_ty));
        }

        for item in &program.items {
            if let Item::FunctionDef {
                name,
                params,
                return_type,
                ..
            } = item
            {
                let ret_ty = self.resolve_type_annotation(return_type);
                let param_types: Vec<NyType> = params
                    .iter()
                    .map(|p| self.resolve_type_annotation(&p.ty))
                    .collect();

                let llvm_param_types: Vec<BasicTypeEnum> = param_types
                    .iter()
                    .map(|t| ny_to_llvm(self.context, t))
                    .collect();

                let param_meta: Vec<_> = llvm_param_types.iter().map(|t| (*t).into()).collect();

                let fn_type = match &ret_ty {
                    NyType::Unit => self.context.void_type().fn_type(&param_meta, false),
                    ty => ny_to_llvm(self.context, ty).fn_type(&param_meta, false),
                };

                let function = self.module.add_function(name, fn_type, None);
                self.functions
                    .insert(name.clone(), (function, param_types, ret_ty));
            }
        }

        // Pass 2a: Compile impl method bodies
        for (qualified_name, params, _, body, _) in &impl_methods {
            let (function, param_types, _) = self.functions[qualified_name].clone();
            let entry = self.context.append_basic_block(function, "entry");
            self.builder.position_at_end(entry);

            let outer_vars = self.variables.clone();
            self.variables.clear();
            let outer_defers = std::mem::take(&mut self.defer_stack);

            for (i, param) in params.iter().enumerate() {
                let ty = &param_types[i];
                let llvm_ty = ny_to_llvm(self.context, ty);
                let alloca = self.builder.build_alloca(llvm_ty, &param.name).unwrap();
                self.builder
                    .build_store(alloca, function.get_nth_param(i as u32).unwrap())
                    .unwrap();
                self.variables
                    .insert(param.name.clone(), (alloca, ty.clone()));
            }

            self.compile_expr(body, &function)?;

            let current_block = self.builder.get_insert_block().unwrap();
            if current_block.get_terminator().is_none() {
                let defers: Vec<(Expr, FunctionValue<'ctx>)> =
                    self.defer_stack.iter().rev().cloned().collect();
                for (defer_body, defer_fn) in &defers {
                    self.compile_expr(defer_body, defer_fn)?;
                }
                self.builder.build_return(None).unwrap();
            }

            self.defer_stack = outer_defers;
            self.variables = outer_vars;
        }

        // Pass 2b: Compile function bodies
        for item in &program.items {
            if let Item::FunctionDef {
                name, params, body, ..
            } = item
            {
                let (function, param_types, _) = self.functions[name].clone();
                let entry = self.context.append_basic_block(function, "entry");
                self.builder.position_at_end(entry);

                // Save outer variables, create fresh scope
                let outer_vars = self.variables.clone();
                self.variables.clear();

                // Allocate parameters
                for (i, param) in params.iter().enumerate() {
                    let ty = &param_types[i];
                    let llvm_ty = ny_to_llvm(self.context, ty);
                    let alloca = self.builder.build_alloca(llvm_ty, &param.name).unwrap();
                    self.builder
                        .build_store(alloca, function.get_nth_param(i as u32).unwrap())
                        .unwrap();
                    self.variables
                        .insert(param.name.clone(), (alloca, ty.clone()));
                }

                // Push function name onto trace stack
                let trace_push = self.get_or_declare_ny_trace_push();
                let fn_name_str = self
                    .builder
                    .build_global_string_ptr(name, "trace_name")
                    .unwrap();
                self.builder
                    .build_call(trace_push, &[fn_name_str.as_pointer_value().into()], "")
                    .unwrap();

                // Save outer defer stack and start fresh for this function
                let outer_defers = std::mem::take(&mut self.defer_stack);

                self.compile_expr(body, &function)?;

                // Emit deferred expressions in LIFO order if no terminator yet
                let current_block = self.builder.get_insert_block().unwrap();
                if current_block.get_terminator().is_none() {
                    let defers: Vec<(Expr, FunctionValue<'ctx>)> =
                        self.defer_stack.iter().rev().cloned().collect();
                    for (defer_body, defer_fn) in &defers {
                        self.compile_expr(defer_body, defer_fn)?;
                    }
                    // Pop trace stack before return
                    let trace_pop = self.get_or_declare_ny_trace_pop();
                    self.builder.build_call(trace_pop, &[], "").unwrap();
                    self.builder.build_return(None).unwrap();
                }

                // Restore outer scope and defers
                self.defer_stack = outer_defers;
                self.variables = outer_vars;
            }
        }

        Ok(())
    }
}
