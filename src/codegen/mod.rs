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
use crate::parser::ast::*;
use types::ny_to_llvm;

pub fn generate(
    program: &Program,
    source_path: &Path,
    output_path: &Path,
    opt_level: u8,
    emit: &str,
    target: &str,
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
            link_executable(&obj_path, output_path)?;
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
                "note: wasm-ld not found. Object file saved as {}",
                obj_path.display()
            );
            eprintln!("  To link: wasm-ld {} -o {} --no-entry --export-all --allow-undefined",
                obj_path.display(), output_path.display());
            eprintln!("  Install: apt install lld-18 && ln -s /usr/bin/wasm-ld-18 /usr/local/bin/wasm-ld");
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
        .arg("-no-pie");

    // Link all runtime C files (hashmap.c, arena.c, etc.)
    for rt_name in &["hashmap.c", "hashmap_generic.c", "arena.c", "channel.c", "threadpool.c", "string.c", "json.c", "tensor.c", "future.c", "gc.c", "chan.c", "error.c"] {
        if let Some(rt_path) = find_runtime_file(rt_name) {
            cmd.arg(rt_path);
        }
    }

    // Libraries MUST come after source files for the linker to resolve symbols
    cmd.arg("-lm").arg("-lc").arg("-lpthread");

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
    pub(super) closure_captures: HashMap<String, (String, Vec<(String, NyType)>)>,
    /// Optimization level (0-3). At O2+, skip bounds checks and stack traces.
    pub(super) opt_level: u8,
    /// Trait definitions: trait_name → [(method_name, param_types, ret_type)]
    pub(super) trait_defs: HashMap<String, Vec<(String, Vec<NyType>, NyType)>>,
    /// VTable globals: "TraitName_for_TypeName" → (global_ptr, method_names_in_order)
    pub(super) vtables: HashMap<String, (PointerValue<'ctx>, Vec<String>)>,
    /// Trait impl mapping: (trait_name, type_name) → method qualified names
    pub(super) trait_impls: HashMap<(String, String), Vec<String>>,
    /// Type aliases: alias_name → resolved NyType
    pub(super) type_aliases: HashMap<String, NyType>,
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
                // Check HashMap<K,V> pattern
                if let Some(inner) = name.strip_prefix("HashMap<").and_then(|s| s.strip_suffix('>')) {
                    if let Some(comma) = inner.find(',') {
                        let k_str = inner[..comma].trim();
                        let v_str = inner[comma + 1..].trim();
                        let k_ty = NyType::from_name(k_str).unwrap_or(NyType::Str);
                        let v_ty = NyType::from_name(v_str).unwrap_or(NyType::I32);
                        return NyType::HashMap(Box::new(k_ty), Box::new(v_ty));
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
                // Try type aliases
                if let Some(ty) = self.type_aliases.get(name) {
                    return ty.clone();
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
            TypeAnnotation::DynTrait { trait_name, .. } => {
                NyType::DynTrait(trait_name.clone())
            }
            TypeAnnotation::Optional { inner, .. } => {
                let inner_ty = self.resolve_type_annotation(inner);
                NyType::Optional(Box::new(inner_ty))
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
            let candidates: Vec<&NyType> = variants
                .iter()
                .filter_map(|(_, p)| p.get(i))
                .collect();
            if candidates.is_empty() {
                field_types.push(self.context.i32_type().into());
            } else {
                field_types.push(types::largest_llvm_type(self.context, &candidates));
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

        // Pass 0a2: Register type aliases
        for item in &program.items {
            if let Item::TypeAlias { name, target, .. } = item {
                let ty = self.resolve_type_annotation(target);
                self.type_aliases.insert(name.clone(), ty);
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

        // Pass 0b2: Collect trait definitions
        for item in &program.items {
            if let Item::TraitDef { name, methods, .. } = item {
                let method_sigs: Vec<(String, Vec<NyType>, NyType)> = methods
                    .iter()
                    .map(|m| {
                        let param_types: Vec<NyType> = m
                            .params
                            .iter()
                            .map(|p| self.resolve_type_annotation(&p.ty))
                            .collect();
                        let ret_ty = self.resolve_type_annotation(&m.return_type);
                        (m.name.clone(), param_types, ret_ty)
                    })
                    .collect();
                self.trait_defs.insert(name.clone(), method_sigs);
            }
        }

        // Pass 0c: Flatten impl methods into top-level functions with qualified names
        let mut impl_methods: Vec<(String, &Vec<Param>, &TypeAnnotation, &Expr, Span)> = Vec::new();
        for item in &program.items {
            if let Item::ImplBlock {
                type_name,
                methods,
                trait_name,
                ..
            } = item
            {
                let mut method_names = Vec::new();
                for method in methods {
                    if let Item::FunctionDef {
                        name,
                        params,
                        return_type,
                        body,
                        span,
                        type_params: _,
                        is_async: _,
                    } = method
                    {
                        let qualified_name = format!("{}_{}", type_name, name);
                        impl_methods.push((qualified_name.clone(), params, return_type, body, *span));
                        method_names.push((name.clone(), qualified_name));
                    }
                }
                // Track trait implementations for vtable generation
                if let Some(tname) = trait_name {
                    let qualified_names: Vec<String> = method_names.iter().map(|(_, q)| q.clone()).collect();
                    self.trait_impls.insert((tname.clone(), type_name.clone()), qualified_names);
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
                is_async,
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

                if *is_async {
                    // Async fn: public wrapper returns ptr (NyFuture*)
                    let ptr_ty = self.context.ptr_type(inkwell::AddressSpace::default());
                    let wrapper_type = ptr_ty.fn_type(&param_meta, false);
                    let wrapper_fn = self.module.add_function(name, wrapper_type, None);
                    self.functions.insert(
                        name.clone(),
                        (wrapper_fn, param_types.clone(), NyType::Future(Box::new(ret_ty.clone()))),
                    );

                    // Also declare the body function (private)
                    let body_name = format!("{}_body", name);
                    let body_type = match &ret_ty {
                        NyType::Unit => self.context.void_type().fn_type(&param_meta, false),
                        ty => ny_to_llvm(self.context, ty).fn_type(&param_meta, false),
                    };
                    let body_fn = self.module.add_function(&body_name, body_type, None);
                    self.functions.insert(
                        body_name,
                        (body_fn, param_types, ret_ty),
                    );
                } else {
                    let fn_type = match &ret_ty {
                        NyType::Unit => self.context.void_type().fn_type(&param_meta, false),
                        ty => ny_to_llvm(self.context, ty).fn_type(&param_meta, false),
                    };

                    let function = self.module.add_function(name, fn_type, None);
                    self.functions
                        .insert(name.clone(), (function, param_types, ret_ty));
                }
            }
        }

        // Pass 1b: Generate vtables for trait implementations
        // Collect trait_impls keys to avoid borrow issues
        let trait_impl_keys: Vec<(String, String)> = self.trait_impls.keys().cloned().collect();
        for (trait_name, type_name) in &trait_impl_keys {
            if let Some(trait_methods) = self.trait_defs.get(trait_name).cloned() {
                let ptr_ty = self.context.ptr_type(inkwell::AddressSpace::default());

                // For each method, create a thunk that takes (*u8, args...) -> ret
                // and loads the concrete struct before forwarding to the real method
                let struct_fields = self.struct_types.get(type_name).cloned().unwrap_or_default();
                let struct_ty = self.get_or_create_llvm_struct_type(type_name, &struct_fields);

                let mut thunk_ptrs: Vec<inkwell::values::PointerValue> = Vec::new();
                let mut ordered_names = Vec::new();

                for (method_name, param_types, ret_ty) in &trait_methods {
                    let qualified = format!("{}_{}", type_name, method_name);
                    if let Some((real_fn, _, _)) = self.functions.get(&qualified).cloned() {
                        // Build thunk signature: (*u8, non-self params...) -> ret
                        let mut thunk_params: Vec<inkwell::types::BasicMetadataTypeEnum> = vec![ptr_ty.into()];
                        for pt in param_types.iter().skip(1) {
                            thunk_params.push(ny_to_llvm(self.context, pt).into());
                        }
                        let thunk_fn_type = if *ret_ty == NyType::Unit {
                            self.context.void_type().fn_type(&thunk_params, false)
                        } else {
                            ny_to_llvm(self.context, ret_ty).fn_type(&thunk_params, false)
                        };
                        let thunk_name = format!("__thunk_{}_{}", type_name, method_name);
                        let thunk_fn = self.module.add_function(&thunk_name, thunk_fn_type, None);

                        let entry = self.context.append_basic_block(thunk_fn, "entry");
                        self.builder.position_at_end(entry);

                        // Load concrete struct from data pointer
                        let data_ptr = thunk_fn.get_nth_param(0).unwrap().into_pointer_value();
                        let struct_val = self.builder.build_load(struct_ty, data_ptr, "self_val").unwrap();

                        // Build call args: loaded struct + forwarded params
                        let mut call_args: Vec<inkwell::values::BasicMetadataValueEnum> = vec![struct_val.into()];
                        for i in 1..thunk_fn.count_params() {
                            call_args.push(thunk_fn.get_nth_param(i).unwrap().into());
                        }

                        let call = self.builder.build_call(real_fn, &call_args, "thunk_call").unwrap();
                        if *ret_ty == NyType::Unit {
                            self.builder.build_return(None).unwrap();
                        } else {
                            let ret_val = call.try_as_basic_value().basic().unwrap();
                            self.builder.build_return(Some(&ret_val)).unwrap();
                        }

                        thunk_ptrs.push(thunk_fn.as_global_value().as_pointer_value());
                        ordered_names.push(method_name.clone());
                    }
                }

                if !thunk_ptrs.is_empty() {
                    let vtable_arr_ty = ptr_ty.array_type(thunk_ptrs.len() as u32);
                    let vtable_name = format!("vtable_{}_for_{}", trait_name, type_name);
                    let vtable_global = self.module.add_global(vtable_arr_ty, None, &vtable_name);
                    let vtable_init = ptr_ty.const_array(&thunk_ptrs);
                    vtable_global.set_initializer(&vtable_init);
                    vtable_global.set_constant(true);

                    let vtable_key = format!("{}_for_{}", trait_name, type_name);
                    self.vtables.insert(
                        vtable_key,
                        (vtable_global.as_pointer_value(), ordered_names),
                    );
                }
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
                name, params, body, is_async, ..
            } = item
            {
                // For async functions, compile the body as _body, then emit the wrapper
                let compile_name = if *is_async {
                    format!("{}_body", name)
                } else {
                    name.clone()
                };
                let (function, param_types, _) = self.functions[&compile_name].clone();
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

                // Initialize GC at the very start of main()
                if name == "main" {
                    let gc_init = self.get_or_declare_ny_gc_init();
                    self.builder.build_call(gc_init, &[], "").unwrap();
                    // Register gc_shutdown to run at exit (covers all return paths)
                    let atexit = self.get_or_declare_atexit();
                    let gc_shutdown = self.get_or_declare_ny_gc_shutdown();
                    self.builder
                        .build_call(atexit, &[gc_shutdown.as_global_value().as_pointer_value().into()], "")
                        .unwrap();
                    // Register async pool shutdown (waits for goroutines before GC runs)
                    // atexit is LIFO: registered after gc_shutdown → runs before gc_shutdown
                    let async_shutdown = self.get_or_declare_ny_async_pool_shutdown();
                    self.builder
                        .build_call(atexit, &[async_shutdown.as_global_value().as_pointer_value().into()], "")
                        .unwrap();
                }

                // Push function name onto trace stack (debug only, skipped at -O2+)
                if self.opt_level < 2 {
                    let trace_push = self.get_or_declare_ny_trace_push();
                    let fn_name_str = self
                        .builder
                        .build_global_string_ptr(name, "trace_name")
                        .unwrap();
                    self.builder
                        .build_call(trace_push, &[fn_name_str.as_pointer_value().into()], "")
                        .unwrap();
                }

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
                    // Pop trace stack before return (debug only)
                    if self.opt_level < 2 {
                        let trace_pop = self.get_or_declare_ny_trace_pop();
                        self.builder.build_call(trace_pop, &[], "").unwrap();
                    }
                    self.builder.build_return(None).unwrap();
                }

                // Restore outer scope and defers
                self.defer_stack = outer_defers;
                self.variables = outer_vars;

                // For async functions: generate the public wrapper
                if *is_async {
                    let (wrapper_fn, wrapper_param_types, _) = self.functions[name].clone();
                    let body_name = format!("{}_body", name);
                    let (body_fn, _, body_ret_ty) = self.functions[&body_name].clone();

                    let entry = self.context.append_basic_block(wrapper_fn, "async_entry");
                    self.builder.position_at_end(entry);

                    let ptr_ty = self.context.ptr_type(inkwell::AddressSpace::default());
                    let i64_ty = self.context.i64_type();

                    // 1. Create future
                    let future_create = self.get_or_declare_ny_future_create();
                    let future_ptr = self
                        .builder
                        .build_call(future_create, &[], "future")
                        .unwrap()
                        .try_as_basic_value()
                        .basic()
                        .unwrap()
                        .into_pointer_value();

                    // 2. Allocate arg struct: [future_ptr, param0, param1, ...]
                    let n_params = wrapper_param_types.len();
                    let arg_fields: Vec<BasicTypeEnum> = std::iter::once(ptr_ty.into())
                        .chain(
                            wrapper_param_types
                                .iter()
                                .map(|t| ny_to_llvm(self.context, t)),
                        )
                        .collect();
                    let arg_struct_ty = self.context.struct_type(&arg_fields, false);
                    let malloc_fn = self.get_or_declare_malloc();
                    let arg_size = arg_struct_ty.size_of().unwrap();
                    let arg_ptr = self
                        .builder
                        .build_call(malloc_fn, &[arg_size.into()], "async_args")
                        .unwrap()
                        .try_as_basic_value()
                        .basic()
                        .unwrap()
                        .into_pointer_value();

                    // Store future ptr at field 0
                    let fp_gep = self
                        .builder
                        .build_struct_gep(arg_struct_ty, arg_ptr, 0, "arg_fp")
                        .unwrap();
                    self.builder.build_store(fp_gep, future_ptr).unwrap();

                    // Store each parameter
                    for i in 0..n_params {
                        let param_gep = self
                            .builder
                            .build_struct_gep(
                                arg_struct_ty,
                                arg_ptr,
                                (i + 1) as u32,
                                &format!("arg_p{}", i),
                            )
                            .unwrap();
                        self.builder
                            .build_store(param_gep, wrapper_fn.get_nth_param(i as u32).unwrap())
                            .unwrap();
                    }

                    // 3. Create thunk function
                    let thunk_name = format!("{}_thunk", name);
                    let thunk_type = ptr_ty.fn_type(&[ptr_ty.into()], false);
                    let thunk_fn = self.module.add_function(&thunk_name, thunk_type, None);

                    let thunk_entry = self.context.append_basic_block(thunk_fn, "thunk_entry");
                    self.builder.position_at_end(thunk_entry);

                    let raw_arg = thunk_fn.get_nth_param(0).unwrap().into_pointer_value();

                    // Load future ptr from arg[0]
                    let tfp_gep = self
                        .builder
                        .build_struct_gep(arg_struct_ty, raw_arg, 0, "t_fp")
                        .unwrap();
                    let t_future = self
                        .builder
                        .build_load(ptr_ty, tfp_gep, "t_future")
                        .unwrap()
                        .into_pointer_value();

                    // Load each parameter from arg struct
                    let mut call_args: Vec<inkwell::values::BasicMetadataValueEnum> = Vec::new();
                    for i in 0..n_params {
                        let p_gep = self
                            .builder
                            .build_struct_gep(
                                arg_struct_ty,
                                raw_arg,
                                (i + 1) as u32,
                                &format!("t_p{}", i),
                            )
                            .unwrap();
                        let p_ty = ny_to_llvm(self.context, &wrapper_param_types[i]);
                        let p_val = self
                            .builder
                            .build_load(p_ty, p_gep, &format!("t_v{}", i))
                            .unwrap();
                        call_args.push(p_val.into());
                    }

                    // Call body function
                    let body_result = self
                        .builder
                        .build_call(body_fn, &call_args, "t_result")
                        .unwrap();

                    // Signal the future with result
                    let future_signal = self.get_or_declare_ny_future_signal();
                    let result_i64 = if body_ret_ty == NyType::Unit {
                        i64_ty.const_int(0, false)
                    } else {
                        let rv = body_result.try_as_basic_value().basic().unwrap();
                        if rv.is_int_value() {
                            self.builder
                                .build_int_s_extend_or_bit_cast(
                                    rv.into_int_value(),
                                    i64_ty,
                                    "t_ext",
                                )
                                .unwrap()
                        } else {
                            // For f64, bitcast to i64
                            self.builder
                                .build_bit_cast(rv, i64_ty, "t_bc")
                                .unwrap()
                                .into_int_value()
                        }
                    };
                    self.builder
                        .build_call(future_signal, &[t_future.into(), result_i64.into()], "")
                        .unwrap();

                    // Free arg struct
                    let free_fn = self.get_or_declare_free();
                    self.builder
                        .build_call(free_fn, &[raw_arg.into()], "")
                        .unwrap();

                    // Return null
                    let null = ptr_ty.const_null();
                    self.builder.build_return(Some(&null)).unwrap();

                    // 4. Back in wrapper: submit thunk to pool
                    self.builder.position_at_end(entry);
                    // Remove the old terminator if any (we need to add more)
                    // Actually entry block has no terminator yet — we're building sequentially

                    // Hmm, we already moved to thunk_entry. Need to go back to async_entry.
                    // The issue: we built the thunk in the middle. Let me restructure.
                    // Actually the thunk is a separate function, so the builder position was set to thunk_entry.
                    // We need to go back to the wrapper's entry block to finish it.

                    // Find the wrapper's entry block (it was named "async_entry")
                    let wrapper_entry = wrapper_fn.get_first_basic_block().unwrap();
                    self.builder.position_at_end(wrapper_entry);

                    // Submit thunk to async pool
                    let async_pool = self.get_or_declare_ny_async_pool();
                    let pool_ptr = self
                        .builder
                        .build_call(async_pool, &[], "pool")
                        .unwrap()
                        .try_as_basic_value()
                        .basic()
                        .unwrap();
                    let pool_submit = self.get_or_declare_ny_pool_submit_arg();
                    self.builder
                        .build_call(
                            pool_submit,
                            &[
                                pool_ptr.into(),
                                thunk_fn.as_global_value().as_pointer_value().into(),
                                arg_ptr.into(),
                            ],
                            "",
                        )
                        .unwrap();

                    // Return future pointer
                    self.builder.build_return(Some(&future_ptr)).unwrap();
                }
            }
        }

        Ok(())
    }
}
