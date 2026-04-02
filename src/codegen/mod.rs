pub mod builtins;
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
use inkwell::values::{BasicValueEnum, FunctionValue, PointerValue};
use inkwell::{AddressSpace, FloatPredicate, IntPredicate, OptimizationLevel};

use crate::common::{CompileError, NyType, Span};
use crate::parser::ast::*;
use types::{ny_to_llvm, str_type};

/// Find free variables in an expression (identifiers not in the given bound set)
fn find_free_vars(expr: &Expr, bound: &[String]) -> Vec<String> {
    let mut free = Vec::new();
    find_free_vars_inner(expr, bound, &mut free);
    free.sort();
    free.dedup();
    free
}

fn find_free_vars_inner(expr: &Expr, bound: &[String], free: &mut Vec<String>) {
    match expr {
        Expr::Ident { name, .. } => {
            if !bound.contains(name) && !free.contains(name) {
                free.push(name.clone());
            }
        }
        Expr::BinOp { lhs, rhs, .. } => {
            find_free_vars_inner(lhs, bound, free);
            find_free_vars_inner(rhs, bound, free);
        }
        Expr::UnaryOp { operand, .. } => {
            find_free_vars_inner(operand, bound, free);
        }
        Expr::Call { args, .. } => {
            for arg in args {
                find_free_vars_inner(arg, bound, free);
            }
        }
        Expr::Block { stmts, tail_expr, .. } => {
            for stmt in stmts {
                match stmt {
                    Stmt::VarDecl { init, .. } | Stmt::ConstDecl { value: init, .. } => {
                        find_free_vars_inner(init, bound, free);
                    }
                    Stmt::ExprStmt { expr, .. } => {
                        find_free_vars_inner(expr, bound, free);
                    }
                    Stmt::Return { value, .. } => {
                        if let Some(v) = value {
                            find_free_vars_inner(v, bound, free);
                        }
                    }
                    _ => {}
                }
            }
            if let Some(te) = tail_expr {
                find_free_vars_inner(te, bound, free);
            }
        }
        Expr::If { condition, then_branch, else_branch, .. } => {
            find_free_vars_inner(condition, bound, free);
            find_free_vars_inner(then_branch, bound, free);
            if let Some(eb) = else_branch {
                find_free_vars_inner(eb, bound, free);
            }
        }
        _ => {}
    }
}

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
    for rt_name in &["hashmap.c", "arena.c"] {
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

struct LoopFrame<'ctx> {
    break_bb: BasicBlock<'ctx>,
    continue_bb: BasicBlock<'ctx>,
}

// ---------------------------------------------------------------------------
// CodeGen
// ---------------------------------------------------------------------------

struct CodeGen<'ctx> {
    context: &'ctx Context,
    module: Module<'ctx>,
    builder: Builder<'ctx>,
    variables: HashMap<String, (PointerValue<'ctx>, NyType)>,
    functions: HashMap<String, (FunctionValue<'ctx>, Vec<NyType>, NyType)>,
    /// Struct name -> ordered list of (field_name, field_type)
    struct_types: HashMap<String, Vec<(String, NyType)>>,
    /// Enum name -> ordered list of (variant_name, payload_types)
    enum_variants: HashMap<String, Vec<(String, Vec<NyType>)>>,
    loop_stack: Vec<LoopFrame<'ctx>>,
    /// Stack of deferred expressions per function scope
    defer_stack: Vec<(Expr, FunctionValue<'ctx>)>,
    /// Closure captures: closure_var_name → (lambda_fn_name, capture_alloca_names)
    /// Each capture has a dedicated alloca "closure_{id}_cap_{name}" that holds
    /// the value at lambda creation time (capture-by-value semantics).
    closure_captures: HashMap<String, (String, Vec<(String, NyType)>)>,
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
    fn enum_struct_type(
        &self,
        enum_name: &str,
    ) -> inkwell::types::StructType<'ctx> {
        let variants = self.enum_variants.get(enum_name).unwrap();
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
    // Infer the NyType of an expression (best-effort, used for codegen)
    // ------------------------------------------------------------------

    fn infer_expr_type(&self, expr: &Expr) -> NyType {
        match expr {
            Expr::Literal { value, .. } => match value {
                LitValue::Int(_) => NyType::I32,
                LitValue::Float(_) => NyType::F64,
                LitValue::Bool(_) => NyType::Bool,
                LitValue::Str(_) => NyType::Str,
            },
            Expr::Ident { name, .. } => {
                if let Some((_, ty)) = self.variables.get(name) {
                    ty.clone()
                } else {
                    NyType::I32
                }
            }
            Expr::BinOp { op, lhs, .. } => match op {
                BinOp::Eq
                | BinOp::Ne
                | BinOp::Lt
                | BinOp::Gt
                | BinOp::Le
                | BinOp::Ge
                | BinOp::And
                | BinOp::Or => NyType::Bool,
                BinOp::Add
                | BinOp::Sub
                | BinOp::Mul
                | BinOp::Div
                | BinOp::Mod
                | BinOp::BitAnd
                | BinOp::BitOr
                | BinOp::BitXor
                | BinOp::Shl
                | BinOp::Shr => self.infer_expr_type(lhs),
            },
            Expr::UnaryOp { op, operand, .. } => match op {
                UnaryOp::Not => NyType::Bool,
                UnaryOp::Neg | UnaryOp::BitNot => self.infer_expr_type(operand),
            },
            Expr::Cast { target_type, .. } => self.resolve_type_annotation(target_type),
            Expr::Call { callee, .. } => {
                if let Some((_, _, ret_ty)) = self.functions.get(callee) {
                    ret_ty.clone()
                } else {
                    // Check if callee is a variable holding a function pointer
                    if let Some((_, var_ty)) = self.variables.get(callee) {
                        if let NyType::Function { ret, .. } = var_ty {
                            return *ret.clone();
                        }
                    }
                    // Also check closure captures for return type
                    if let Some((lambda_name, _)) = self.closure_captures.get(callee) {
                        if let Some((_, _, ret_ty)) = self.functions.get(lambda_name) {
                            return ret_ty.clone();
                        }
                    }
                    // Use builtin registry for return types
                    builtins::builtin_return_type(callee, &[])
                        .unwrap_or(NyType::Unit)
                }
            }
            Expr::If { then_branch, .. } => self.infer_expr_type(then_branch),
            Expr::Block { tail_expr, .. } => {
                if let Some(expr) = tail_expr {
                    self.infer_expr_type(expr)
                } else {
                    NyType::Unit
                }
            }
            Expr::ArrayLit { elements, .. } => {
                let size = elements.len();
                let elem_ty = if let Some(first) = elements.first() {
                    self.infer_expr_type(first)
                } else {
                    NyType::I32
                };
                NyType::Array {
                    elem: Box::new(elem_ty),
                    size,
                }
            }
            Expr::Index { object, .. } => {
                let obj_ty = self.infer_expr_type(object);
                match obj_ty {
                    NyType::Array { elem, .. } => *elem,
                    _ => NyType::I32,
                }
            }
            Expr::FieldAccess { object, field, .. } => {
                let obj_ty = self.infer_expr_type(object);
                let struct_ty = match &obj_ty {
                    NyType::Pointer(inner) => inner.as_ref(),
                    other => other,
                };
                if let Some(ft) = struct_ty.field_type(field) {
                    ft.clone()
                } else {
                    NyType::I32
                }
            }
            Expr::StructInit { name, .. } => {
                if let Some(fields) = self.struct_types.get(name) {
                    NyType::Struct {
                        name: name.clone(),
                        fields: fields.clone(),
                    }
                } else {
                    NyType::I32
                }
            }
            Expr::AddrOf { operand, .. } => {
                let inner_ty = self.infer_expr_type(operand);
                NyType::Pointer(Box::new(inner_ty))
            }
            Expr::Deref { operand, .. } => {
                let ptr_ty = self.infer_expr_type(operand);
                match ptr_ty {
                    NyType::Pointer(inner) => *inner,
                    _ => NyType::I32,
                }
            }
            Expr::MethodCall { object, method, .. } => {
                let obj_ty = self.infer_expr_type(object);
                match &obj_ty {
                    NyType::Vec(elem) => match method.as_str() {
                        "len" => NyType::I64,
                        "get" | "pop" => *elem.clone(),
                        _ => NyType::Unit,
                    },
                    NyType::Slice(_) => match method.as_str() {
                        "len" => NyType::I64,
                        _ => NyType::Unit,
                    },
                    NyType::Str => match method.as_str() {
                        "len" => NyType::I64,
                        "substr" => NyType::Str,
                        _ => NyType::Unit,
                    },
                    _ => {
                        // Look up by method name first
                        if let Some((_, _, ret_ty)) = self.functions.get(method) {
                            return ret_ty.clone();
                        }
                        // Try TypeName_method convention
                        let type_name = match &obj_ty {
                            NyType::Struct { name, .. } => name.clone(),
                            NyType::Pointer(inner) => match inner.as_ref() {
                                NyType::Struct { name, .. } => name.clone(),
                                _ => String::new(),
                            },
                            _ => String::new(),
                        };
                        if !type_name.is_empty() {
                            let qualified = format!("{}_{}", type_name, method);
                            if let Some((_, _, ret_ty)) = self.functions.get(&qualified) {
                                return ret_ty.clone();
                            }
                        }
                        NyType::Unit
                    }
                }
            }
            Expr::Match { arms, .. } => {
                if let Some(first_arm) = arms.first() {
                    self.infer_expr_type(&first_arm.body)
                } else {
                    NyType::Unit
                }
            }
            Expr::TupleLit { elements, .. } => {
                let elem_types: Vec<NyType> =
                    elements.iter().map(|e| self.infer_expr_type(e)).collect();
                NyType::Tuple(elem_types)
            }
            Expr::TupleIndex { object, index, .. } => {
                let obj_ty = self.infer_expr_type(object);
                match obj_ty {
                    NyType::Tuple(elems) => elems.get(*index).cloned().unwrap_or(NyType::I32),
                    _ => NyType::I32,
                }
            }
            Expr::Try { operand, .. } => {
                let op_ty = self.infer_expr_type(operand);
                match &op_ty {
                    NyType::Enum { variants, .. } => {
                        if let Some((_, payload)) = variants.first() {
                            if payload.is_empty() {
                                NyType::Unit
                            } else {
                                payload[0].clone()
                            }
                        } else {
                            NyType::I32
                        }
                    }
                    _ => NyType::I32,
                }
            }
            Expr::Lambda {
                params,
                return_type,
                ..
            } => {
                let param_types: Vec<NyType> = params
                    .iter()
                    .map(|p| self.resolve_type_annotation(&p.ty))
                    .collect();
                let ret_ty = self.resolve_type_annotation(return_type);
                NyType::Function {
                    params: param_types,
                    ret: Box::new(ret_ty),
                }
            }
            Expr::RangeIndex { object, .. } => {
                let obj_ty = self.infer_expr_type(object);
                match obj_ty {
                    NyType::Array { elem, .. } => NyType::Slice(elem),
                    NyType::Slice(elem) => NyType::Slice(elem),
                    _ => NyType::Slice(Box::new(NyType::I32)),
                }
            }
            Expr::EnumVariant { enum_name, .. } => {
                if let Some(variant_defs) = self.enum_variants.get(enum_name) {
                    NyType::Enum {
                        name: enum_name.clone(),
                        variants: variant_defs.clone(),
                    }
                } else {
                    NyType::I32
                }
            }
        }
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
                    let param_meta: Vec<_> =
                        llvm_param_types.iter().map(|t| (*t).into()).collect();

                    let fn_type = match &ret_ty {
                        NyType::Unit => {
                            self.context.void_type().fn_type(&param_meta, ext_fn.variadic)
                        }
                        ty => ny_to_llvm(self.context, ty)
                            .fn_type(&param_meta, ext_fn.variadic),
                    };

                    let function =
                        self.module.add_function(&ext_fn.name, fn_type, None);
                    self.functions.insert(
                        ext_fn.name.clone(),
                        (function, param_types, ret_ty),
                    );
                }
            }
        }

        // Pass 0c: Flatten impl methods into top-level functions with qualified names
        let mut impl_methods: Vec<(String, &Vec<Param>, &TypeAnnotation, &Expr, Span)> = Vec::new();
        for item in &program.items {
            if let Item::ImplBlock {
                type_name, methods, trait_name: _, ..
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
                    self.builder.build_return(None).unwrap();
                }

                // Restore outer scope and defers
                self.defer_stack = outer_defers;
                self.variables = outer_vars;
            }
        }

        Ok(())
    }

    // ------------------------------------------------------------------
    // Compile expressions
    // ------------------------------------------------------------------

    fn compile_expr(
        &mut self,
        expr: &Expr,
        function: &FunctionValue<'ctx>,
    ) -> Result<Option<BasicValueEnum<'ctx>>, Vec<CompileError>> {
        match expr {
            // ---- Literals ----
            Expr::Literal { value, .. } => match value {
                LitValue::Int(n) => {
                    let val = self.context.i32_type().const_int(*n as u64, true);
                    Ok(Some(val.into()))
                }
                LitValue::Float(f) => {
                    let val = self.context.f64_type().const_float(*f);
                    Ok(Some(val.into()))
                }
                LitValue::Bool(b) => {
                    let val = self
                        .context
                        .bool_type()
                        .const_int(if *b { 1 } else { 0 }, false);
                    Ok(Some(val.into()))
                }
                LitValue::Str(s) => {
                    // Build a string literal as a {ptr, len} struct value
                    let str_val = self.build_str_literal(s);
                    Ok(Some(str_val))
                }
            },

            // ---- Identifiers ----
            Expr::Ident { name, .. } => {
                if let Some((ptr, ty)) = self.variables.get(name) {
                    let llvm_ty = ny_to_llvm(self.context, ty);
                    let val = self.builder.build_load(llvm_ty, *ptr, name).unwrap();
                    Ok(Some(val))
                } else if let Some((func, _, _)) = self.functions.get(name) {
                    // Function name used as value → return function pointer
                    Ok(Some(func.as_global_value().as_pointer_value().into()))
                } else {
                    Ok(None)
                }
            }

            // ---- Binary operations ----
            Expr::BinOp { op, lhs, rhs, .. } => {
                let lhs_val = self.compile_expr(lhs, function)?.unwrap();
                let rhs_val = self.compile_expr(rhs, function)?.unwrap();
                let result = self.compile_binop(*op, lhs_val, rhs_val)?;
                Ok(Some(result))
            }

            // ---- Unary operations ----
            Expr::UnaryOp { op, operand, .. } => {
                let val = self.compile_expr(operand, function)?.unwrap();
                let result = self.compile_unaryop(*op, val)?;
                Ok(Some(result))
            }

            // ---- Function calls (including print/println builtins) ----
            Expr::Call { callee, args, .. } => {
                // Handle print/println builtins
                if callee == "print" || callee == "println" {
                    self.compile_print_call(callee, args, function)?;
                    return Ok(None);
                }

                // Handle alloc(Type) builtin — returns *Type via malloc
                if callee == "alloc" {
                    // alloc expects exactly 1 argument which evaluates to a size
                    let size_val = self
                        .compile_expr(&args[0], function)?
                        .unwrap()
                        .into_int_value();
                    let size_i64 = self
                        .builder
                        .build_int_z_extend_or_bit_cast(
                            size_val,
                            self.context.i64_type(),
                            "alloc_size",
                        )
                        .unwrap();
                    let malloc_fn = self.get_or_declare_malloc();
                    let ptr = self
                        .builder
                        .build_call(malloc_fn, &[size_i64.into()], "alloc_ptr")
                        .unwrap()
                        .try_as_basic_value()
                        .basic()
                        .unwrap();
                    return Ok(Some(ptr));
                }

                // Handle free(ptr) builtin
                if callee == "free" {
                    let ptr_val = self.compile_expr(&args[0], function)?.unwrap();
                    let free_fn = self.get_or_declare_free();
                    self.builder
                        .build_call(free_fn, &[ptr_val.into()], "")
                        .unwrap();
                    return Ok(None);
                }

                // Handle fopen(path_str, mode_str) -> FILE*
                if callee == "fopen" {
                    let path_val = self.compile_expr(&args[0], function)?.unwrap();
                    let mode_val = self.compile_expr(&args[1], function)?.unwrap();
                    // Extract ptr from {ptr, len} str structs
                    let path_ptr = self
                        .builder
                        .build_extract_value(path_val.into_struct_value(), 0, "path_ptr")
                        .unwrap();
                    let mode_ptr = self
                        .builder
                        .build_extract_value(mode_val.into_struct_value(), 0, "mode_ptr")
                        .unwrap();
                    let fopen_fn = self.get_or_declare_fopen();
                    let result = self
                        .builder
                        .build_call(fopen_fn, &[path_ptr.into(), mode_ptr.into()], "fp")
                        .unwrap()
                        .try_as_basic_value()
                        .basic()
                        .unwrap();
                    return Ok(Some(result));
                }

                // Handle fclose(fp) -> i32
                if callee == "fclose" {
                    let fp = self.compile_expr(&args[0], function)?.unwrap();
                    let fclose_fn = self.get_or_declare_fclose();
                    let result = self
                        .builder
                        .build_call(fclose_fn, &[fp.into()], "fclose_ret")
                        .unwrap()
                        .try_as_basic_value()
                        .basic()
                        .unwrap();
                    return Ok(Some(result));
                }

                // Handle fwrite_str(fp, str) -> bytes written
                if callee == "fwrite_str" {
                    let fp = self.compile_expr(&args[0], function)?.unwrap();
                    let str_val = self.compile_expr(&args[1], function)?.unwrap();
                    let ptr = self
                        .builder
                        .build_extract_value(str_val.into_struct_value(), 0, "str_ptr")
                        .unwrap();
                    let len = self
                        .builder
                        .build_extract_value(str_val.into_struct_value(), 1, "str_len")
                        .unwrap();
                    let fwrite_fn = self.get_or_declare_fwrite();
                    let one = self.context.i64_type().const_int(1, false);
                    let result = self
                        .builder
                        .build_call(
                            fwrite_fn,
                            &[ptr.into(), one.into(), len.into(), fp.into()],
                            "fwrite_ret",
                        )
                        .unwrap()
                        .try_as_basic_value()
                        .basic()
                        .unwrap();
                    // Cast size_t result to i32
                    let i32_result = self
                        .builder
                        .build_int_truncate(
                            result.into_int_value(),
                            self.context.i32_type(),
                            "fwrite_i32",
                        )
                        .unwrap();
                    return Ok(Some(i32_result.into()));
                }

                // Handle fread_byte(fp) -> i32 (fgetc)
                if callee == "fread_byte" {
                    let fp = self.compile_expr(&args[0], function)?.unwrap();
                    let fgetc_fn = self.get_or_declare_fgetc();
                    let result = self
                        .builder
                        .build_call(fgetc_fn, &[fp.into()], "fgetc_ret")
                        .unwrap()
                        .try_as_basic_value()
                        .basic()
                        .unwrap();
                    return Ok(Some(result));
                }

                // Handle exit(code)
                if callee == "exit" {
                    let code = self.compile_expr(&args[0], function)?.unwrap();
                    let exit_fn = self.get_or_declare_exit();
                    self.builder
                        .build_call(exit_fn, &[code.into()], "")
                        .unwrap();
                    self.builder.build_unreachable().unwrap();
                    return Ok(None);
                }

                // Arena builtins — call C runtime functions
                if callee == "arena_new" {
                    let size_hint = self.compile_expr(&args[0], function)?.unwrap();
                    let size_i64 = self.builder.build_int_s_extend_or_bit_cast(
                        size_hint.into_int_value(), self.context.i64_type(), "arena_size"
                    ).unwrap();
                    let arena_fn = self.get_or_declare_ny_arena_new();
                    let ptr = self.builder.build_call(arena_fn, &[size_i64.into()], "arena").unwrap()
                        .try_as_basic_value().basic().unwrap();
                    return Ok(Some(ptr));
                }
                if callee == "arena_alloc" {
                    let arena = self.compile_expr(&args[0], function)?.unwrap();
                    let size = self.compile_expr(&args[1], function)?.unwrap();
                    let size_i64 = self.builder.build_int_s_extend_or_bit_cast(
                        size.into_int_value(), self.context.i64_type(), "alloc_size"
                    ).unwrap();
                    let alloc_fn = self.get_or_declare_ny_arena_alloc();
                    let ptr = self.builder.build_call(alloc_fn, &[arena.into(), size_i64.into()], "arena_ptr").unwrap()
                        .try_as_basic_value().basic().unwrap();
                    return Ok(Some(ptr));
                }
                if callee == "arena_free" {
                    let arena = self.compile_expr(&args[0], function)?.unwrap();
                    let free_fn = self.get_or_declare_ny_arena_free();
                    self.builder.build_call(free_fn, &[arena.into()], "").unwrap();
                    return Ok(None);
                }
                if callee == "arena_reset" {
                    let arena = self.compile_expr(&args[0], function)?.unwrap();
                    let reset_fn = self.get_or_declare_ny_arena_reset();
                    self.builder.build_call(reset_fn, &[arena.into()], "").unwrap();
                    return Ok(None);
                }
                if callee == "arena_bytes_used" {
                    let arena = self.compile_expr(&args[0], function)?.unwrap();
                    let bytes_fn = self.get_or_declare_ny_arena_bytes_used();
                    let result = self.builder.build_call(bytes_fn, &[arena.into()], "arena_used").unwrap()
                        .try_as_basic_value().basic().unwrap();
                    return Ok(Some(result));
                }

                // Handle map_new() → calls ny_map_new() from runtime
                if callee == "map_new" {
                    let map_new_fn = self.get_or_declare_ny_map_new();
                    let ptr = self
                        .builder
                        .build_call(map_new_fn, &[], "map_ptr")
                        .unwrap()
                        .try_as_basic_value()
                        .basic()
                        .unwrap();
                    return Ok(Some(ptr));
                }

                // Handle map_insert(m, key_str, value_i32)
                if callee == "map_insert" {
                    let map_ptr = self.compile_expr(&args[0], function)?.unwrap();
                    let key_val = self
                        .compile_expr(&args[1], function)?
                        .unwrap()
                        .into_struct_value();
                    let key_ptr = self
                        .builder
                        .build_extract_value(key_val, 0, "key_ptr")
                        .unwrap();
                    let key_len = self
                        .builder
                        .build_extract_value(key_val, 1, "key_len")
                        .unwrap();
                    let value = self.compile_expr(&args[2], function)?.unwrap();
                    let value_i64 = self
                        .builder
                        .build_int_s_extend(
                            value.into_int_value(),
                            self.context.i64_type(),
                            "val_i64",
                        )
                        .unwrap();
                    let insert_fn = self.get_or_declare_ny_map_insert();
                    self.builder
                        .build_call(
                            insert_fn,
                            &[map_ptr.into(), key_ptr.into(), key_len.into(), value_i64.into()],
                            "",
                        )
                        .unwrap();
                    return Ok(None);
                }

                // Handle map_get(m, key_str) -> i32
                if callee == "map_get" {
                    let map_ptr = self.compile_expr(&args[0], function)?.unwrap();
                    let key_val = self
                        .compile_expr(&args[1], function)?
                        .unwrap()
                        .into_struct_value();
                    let key_ptr = self
                        .builder
                        .build_extract_value(key_val, 0, "key_ptr")
                        .unwrap();
                    let key_len = self
                        .builder
                        .build_extract_value(key_val, 1, "key_len")
                        .unwrap();
                    let get_fn = self.get_or_declare_ny_map_get();
                    let result = self
                        .builder
                        .build_call(get_fn, &[map_ptr.into(), key_ptr.into(), key_len.into()], "map_val")
                        .unwrap()
                        .try_as_basic_value()
                        .basic()
                        .unwrap();
                    let result_i32 = self
                        .builder
                        .build_int_truncate(
                            result.into_int_value(),
                            self.context.i32_type(),
                            "map_i32",
                        )
                        .unwrap();
                    return Ok(Some(result_i32.into()));
                }

                // Handle map_contains(m, key_str) -> bool
                if callee == "map_contains" {
                    let map_ptr = self.compile_expr(&args[0], function)?.unwrap();
                    let key_val = self
                        .compile_expr(&args[1], function)?
                        .unwrap()
                        .into_struct_value();
                    let key_ptr = self
                        .builder
                        .build_extract_value(key_val, 0, "key_ptr")
                        .unwrap();
                    let key_len = self
                        .builder
                        .build_extract_value(key_val, 1, "key_len")
                        .unwrap();
                    let contains_fn = self.get_or_declare_ny_map_contains();
                    let result = self
                        .builder
                        .build_call(
                            contains_fn,
                            &[map_ptr.into(), key_ptr.into(), key_len.into()],
                            "map_has",
                        )
                        .unwrap()
                        .try_as_basic_value()
                        .basic()
                        .unwrap();
                    let bool_val = self
                        .builder
                        .build_int_compare(
                            IntPredicate::NE,
                            result.into_int_value(),
                            self.context.i32_type().const_zero(),
                            "has_bool",
                        )
                        .unwrap();
                    return Ok(Some(bool_val.into()));
                }

                // Handle map_len(m) -> i64
                if callee == "map_len" {
                    let map_ptr = self.compile_expr(&args[0], function)?.unwrap();
                    let len_fn = self.get_or_declare_ny_map_len();
                    let result = self
                        .builder
                        .build_call(len_fn, &[map_ptr.into()], "map_len")
                        .unwrap()
                        .try_as_basic_value()
                        .basic()
                        .unwrap();
                    return Ok(Some(result));
                }

                // SIMD builtins
                if callee == "simd_splat_f32x4" {
                    let raw_scalar = self
                        .compile_expr(&args[0], function)?
                        .unwrap()
                        .into_float_value();
                    // Truncate to f32 if the literal is f64
                    let scalar = if raw_scalar.get_type() == self.context.f64_type() {
                        self.builder
                            .build_float_trunc(raw_scalar, self.context.f32_type(), "f32_trunc")
                            .unwrap()
                    } else {
                        raw_scalar
                    };
                    let vec_ty = self.context.f32_type().vec_type(4);
                    let mut vec_val = vec_ty.get_undef();
                    for i in 0..4u32 {
                        let idx = self.context.i32_type().const_int(i as u64, false);
                        vec_val = self
                            .builder
                            .build_insert_element(vec_val, scalar, idx, &format!("splat_{}", i))
                            .unwrap();
                    }
                    return Ok(Some(vec_val.into()));
                }
                if callee == "simd_splat_f32x8" {
                    let raw_scalar = self
                        .compile_expr(&args[0], function)?
                        .unwrap()
                        .into_float_value();
                    let scalar = if raw_scalar.get_type() == self.context.f64_type() {
                        self.builder
                            .build_float_trunc(raw_scalar, self.context.f32_type(), "f32_trunc")
                            .unwrap()
                    } else {
                        raw_scalar
                    };
                    let vec_ty = self.context.f32_type().vec_type(8);
                    let mut vec_val = vec_ty.get_undef();
                    for i in 0..8u32 {
                        let idx = self.context.i32_type().const_int(i as u64, false);
                        vec_val = self
                            .builder
                            .build_insert_element(vec_val, scalar, idx, &format!("splat_{}", i))
                            .unwrap();
                    }
                    return Ok(Some(vec_val.into()));
                }
                if callee == "simd_reduce_add_f32" {
                    let vec_val = self.compile_expr(&args[0], function)?.unwrap();
                    let vec = vec_val.into_vector_value();
                    let lanes = vec.get_type().get_size();
                    let mut sum = self
                        .builder
                        .build_extract_element(
                            vec,
                            self.context.i32_type().const_zero(),
                            "lane_0",
                        )
                        .unwrap()
                        .into_float_value();
                    for i in 1..lanes {
                        let idx = self.context.i32_type().const_int(i as u64, false);
                        let lane = self
                            .builder
                            .build_extract_element(vec, idx, &format!("lane_{}", i))
                            .unwrap()
                            .into_float_value();
                        sum = self
                            .builder
                            .build_float_add(sum, lane, &format!("hadd_{}", i))
                            .unwrap();
                    }
                    return Ok(Some(sum.into()));
                }

                // SIMD load: simd_load_f32x4(ptr, offset) → load 4 consecutive f32
                if callee == "simd_load_f32x4" || callee == "simd_load_f32x8" {
                    let lanes: u32 = if callee == "simd_load_f32x4" { 4 } else { 8 };
                    let ptr = self.compile_expr(&args[0], function)?.unwrap().into_pointer_value();
                    let offset = self.compile_expr(&args[1], function)?.unwrap().into_int_value();
                    let offset_i64 = self.builder
                        .build_int_s_extend_or_bit_cast(offset, self.context.i64_type(), "off64")
                        .unwrap();
                    // GEP to ptr + offset (byte-level, offset is in elements)
                    let f32_ty = self.context.f32_type();
                    let elem_ptr = unsafe {
                        self.builder.build_in_bounds_gep(f32_ty, ptr, &[offset_i64], "simd_ptr").unwrap()
                    };
                    // Bitcast to vector pointer and load
                    let vec_ty = f32_ty.vec_type(lanes);
                    let vec_val = self.builder.build_load(vec_ty, elem_ptr, "simd_load").unwrap();
                    return Ok(Some(vec_val));
                }

                // SIMD store: simd_store_f32x4(ptr, offset, vec)
                if callee == "simd_store_f32x4" || callee == "simd_store_f32x8" {
                    let ptr = self.compile_expr(&args[0], function)?.unwrap().into_pointer_value();
                    let offset = self.compile_expr(&args[1], function)?.unwrap().into_int_value();
                    let vec_val = self.compile_expr(&args[2], function)?.unwrap();
                    let offset_i64 = self.builder
                        .build_int_s_extend_or_bit_cast(offset, self.context.i64_type(), "off64")
                        .unwrap();
                    let f32_ty = self.context.f32_type();
                    let elem_ptr = unsafe {
                        self.builder.build_in_bounds_gep(f32_ty, ptr, &[offset_i64], "store_ptr").unwrap()
                    };
                    self.builder.build_store(elem_ptr, vec_val).unwrap();
                    return Ok(None);
                }

                // to_str(val) → converts any type to str via snprintf
                if callee == "to_str" {
                    let arg_ty = self.infer_expr_type(&args[0]);
                    let val = self.compile_expr(&args[0], function)?.unwrap();

                    let buf_size = self.context.i64_type().const_int(64, false);
                    let malloc_fn = self.get_or_declare_malloc();
                    let buf_ptr = self.builder
                        .build_call(malloc_fn, &[buf_size.into()], "ts_buf")
                        .unwrap().try_as_basic_value().basic().unwrap().into_pointer_value();
                    let snprintf_fn = self.get_or_declare_snprintf();

                    let fmt_str = match &arg_ty {
                        NyType::I32 => "%d",
                        NyType::I64 => "%ld",
                        t if t.is_integer() => "%ld",
                        NyType::F32 | NyType::F64 => "%f",
                        NyType::Bool => "%s",
                        NyType::Str => "%s", // identity for strings
                        _ => "%d",
                    };
                    let fmt = self.builder.build_global_string_ptr(fmt_str, "ts_fmt").unwrap();

                    let print_val: BasicValueEnum = if arg_ty == NyType::Bool {
                        let b = val.into_int_value();
                        let ts = self.builder.build_global_string_ptr("true", "ts_t").unwrap();
                        let fs = self.builder.build_global_string_ptr("false", "ts_f").unwrap();
                        self.builder.build_select(b, ts.as_pointer_value(), fs.as_pointer_value(), "ts_sel").unwrap()
                    } else if arg_ty == NyType::Str {
                        let sv = val.into_struct_value();
                        self.builder.build_extract_value(sv, 0, "ts_ptr").unwrap()
                    } else if arg_ty.is_integer() && arg_ty != NyType::I32 {
                        let ext = self.builder.build_int_s_extend(val.into_int_value(), self.context.i64_type(), "ts_ext").unwrap();
                        ext.into()
                    } else {
                        val
                    };

                    self.builder.build_call(
                        snprintf_fn,
                        &[buf_ptr.into(), buf_size.into(), fmt.as_pointer_value().into(), print_val.into()],
                        "",
                    ).unwrap();

                    let strlen_fn = self.get_or_declare_strlen();
                    let len = self.builder
                        .build_call(strlen_fn, &[buf_ptr.into()], "ts_len")
                        .unwrap().try_as_basic_value().basic().unwrap().into_int_value();

                    let str_ty = str_type(self.context);
                    let str_val = str_ty.const_zero();
                    let str_val = self.builder.build_insert_value(str_val, buf_ptr, 0, "ts_p").unwrap();
                    let str_val = self.builder.build_insert_value(str_val, len, 1, "ts_l").unwrap();
                    return Ok(Some(str_val.into_struct_value().into()));
                }

                // Thread builtins
                if callee == "thread_spawn" {
                    // thread_spawn(fn_ptr) → spawns a pthread, returns thread handle
                    let fn_ptr = self.compile_expr(&args[0], function)?.unwrap();
                    let pthread_create = self.get_or_declare_pthread_create();
                    // Allocate space for pthread_t (i64)
                    let handle_alloca = self.builder
                        .build_alloca(self.context.i64_type(), "thread_handle")
                        .unwrap();
                    let null = self.context.ptr_type(AddressSpace::default()).const_null();
                    self.builder.build_call(
                        pthread_create,
                        &[handle_alloca.into(), null.into(), fn_ptr.into(), null.into()],
                        "spawn",
                    ).unwrap();
                    let handle = self.builder
                        .build_load(self.context.i64_type(), handle_alloca, "tid")
                        .unwrap();
                    return Ok(Some(handle));
                }

                if callee == "thread_join" {
                    let handle = self.compile_expr(&args[0], function)?.unwrap();
                    let pthread_join = self.get_or_declare_pthread_join();
                    let null = self.context.ptr_type(AddressSpace::default()).const_null();
                    self.builder.build_call(
                        pthread_join,
                        &[handle.into(), null.into()],
                        "join",
                    ).unwrap();
                    return Ok(None);
                }

                // Handle vec_new() — creates empty Vec with correct elem_size
                if callee == "vec_new" {
                    let initial_cap: u64 = 8;
                    // Default elem_size 8 (overridden in VarDecl when type annotation present)
                    let elem_size_val = self.context.i64_type().const_int(8, false);
                    let alloc_size = self
                        .builder
                        .build_int_mul(
                            self.context.i64_type().const_int(initial_cap, false),
                            elem_size_val,
                            "vec_alloc_size",
                        )
                        .unwrap();
                    let malloc_fn = self.get_or_declare_malloc();
                    let data_ptr = self
                        .builder
                        .build_call(malloc_fn, &[alloc_size.into()], "vec_data")
                        .unwrap()
                        .try_as_basic_value()
                        .basic()
                        .unwrap()
                        .into_pointer_value();
                    let vec_ty = ny_to_llvm(self.context, &NyType::Vec(Box::new(NyType::I32)));
                    let vec_val = vec_ty.into_struct_type().const_zero();
                    let vec_val = self
                        .builder
                        .build_insert_value(vec_val, data_ptr, 0, "vec_p")
                        .unwrap();
                    let vec_val = self
                        .builder
                        .build_insert_value(
                            vec_val,
                            self.context.i64_type().const_zero(),
                            1,
                            "vec_l",
                        )
                        .unwrap();
                    let vec_val = self
                        .builder
                        .build_insert_value(
                            vec_val,
                            self.context.i64_type().const_int(initial_cap, false),
                            2,
                            "vec_c",
                        )
                        .unwrap();
                    let vec_val = self
                        .builder
                        .build_insert_value(vec_val, elem_size_val, 3, "vec_es")
                        .unwrap();
                    return Ok(Some(vec_val.into_struct_value().into()));
                }

                // Handle read_line() — reads a line from stdin using fgets
                if callee == "read_line" {
                    // Allocate a 1024-byte buffer
                    let buf_size = self.context.i64_type().const_int(1024, false);
                    let malloc_fn = self.get_or_declare_malloc();
                    let buf_ptr = self
                        .builder
                        .build_call(malloc_fn, &[buf_size.into()], "line_buf")
                        .unwrap()
                        .try_as_basic_value()
                        .basic()
                        .unwrap()
                        .into_pointer_value();

                    // Call fgets(buf, 1024, stdin)
                    let fgets_fn = self.get_or_declare_fgets();
                    let stdin_fn = self.get_or_declare_stdin();
                    let stdin_val = self
                        .builder
                        .build_load(
                            self.context.ptr_type(AddressSpace::default()),
                            stdin_fn.as_pointer_value(),
                            "stdin_val",
                        )
                        .unwrap();
                    let size_i32 = self.context.i32_type().const_int(1024, false);
                    self.builder
                        .build_call(
                            fgets_fn,
                            &[buf_ptr.into(), size_i32.into(), stdin_val.into()],
                            "fgets_ret",
                        )
                        .unwrap();

                    // Compute length with strlen
                    let strlen_fn = self.get_or_declare_strlen();
                    let len = self
                        .builder
                        .build_call(strlen_fn, &[buf_ptr.into()], "line_len")
                        .unwrap()
                        .try_as_basic_value()
                        .basic()
                        .unwrap()
                        .into_int_value();

                    // Strip trailing newline: if buf[len-1] == '\n', len--
                    let one = self.context.i64_type().const_int(1, false);
                    let len_minus_1 = self
                        .builder
                        .build_int_sub(len, one, "len_m1")
                        .unwrap();
                    let last_char_ptr = unsafe {
                        self.builder
                            .build_in_bounds_gep(
                                self.context.i8_type(),
                                buf_ptr,
                                &[len_minus_1],
                                "last_ptr",
                            )
                            .unwrap()
                    };
                    let last_char = self
                        .builder
                        .build_load(self.context.i8_type(), last_char_ptr, "last_char")
                        .unwrap()
                        .into_int_value();
                    let newline = self.context.i8_type().const_int(10, false); // '\n'
                    let is_newline = self
                        .builder
                        .build_int_compare(IntPredicate::EQ, last_char, newline, "is_nl")
                        .unwrap();
                    let final_len = self
                        .builder
                        .build_select(is_newline, len_minus_1, len, "final_len")
                        .unwrap()
                        .into_int_value();

                    // Build {ptr, len} str
                    let str_ty = str_type(self.context);
                    let str_val = str_ty.const_zero();
                    let str_val = self
                        .builder
                        .build_insert_value(str_val, buf_ptr, 0, "rl_ptr")
                        .unwrap();
                    let str_val = self
                        .builder
                        .build_insert_value(str_val, final_len, 1, "rl_len")
                        .unwrap();
                    return Ok(Some(str_val.into_struct_value().into()));
                }

                // Handle str_to_int(s) — wraps atoi
                if callee == "str_to_int" {
                    let str_val = self
                        .compile_expr(&args[0], function)?
                        .unwrap()
                        .into_struct_value();
                    let ptr = self
                        .builder
                        .build_extract_value(str_val, 0, "s2i_ptr")
                        .unwrap();
                    let atoi_fn = self.get_or_declare_atoi();
                    let result = self
                        .builder
                        .build_call(atoi_fn, &[ptr.into()], "atoi_ret")
                        .unwrap()
                        .try_as_basic_value()
                        .basic()
                        .unwrap();
                    return Ok(Some(result));
                }

                // Handle int_to_str(n) — wraps snprintf
                if callee == "int_to_str" {
                    let int_val = self.compile_expr(&args[0], function)?.unwrap();

                    // Allocate buffer for the string representation
                    let buf_size = self.context.i64_type().const_int(32, false);
                    let malloc_fn = self.get_or_declare_malloc();
                    let buf_ptr = self
                        .builder
                        .build_call(malloc_fn, &[buf_size.into()], "i2s_buf")
                        .unwrap()
                        .try_as_basic_value()
                        .basic()
                        .unwrap()
                        .into_pointer_value();

                    // snprintf(buf, 32, "%d", val)
                    let snprintf_fn = self.get_or_declare_snprintf();
                    let fmt = self
                        .builder
                        .build_global_string_ptr("%d", "i2s_fmt")
                        .unwrap();
                    let size_i64 = self.context.i64_type().const_int(32, false);
                    self.builder
                        .build_call(
                            snprintf_fn,
                            &[
                                buf_ptr.into(),
                                size_i64.into(),
                                fmt.as_pointer_value().into(),
                                int_val.into(),
                            ],
                            "snprintf_ret",
                        )
                        .unwrap();

                    // Get length with strlen
                    let strlen_fn = self.get_or_declare_strlen();
                    let len = self
                        .builder
                        .build_call(strlen_fn, &[buf_ptr.into()], "i2s_len")
                        .unwrap()
                        .try_as_basic_value()
                        .basic()
                        .unwrap()
                        .into_int_value();

                    // Build {ptr, len} str
                    let str_ty = str_type(self.context);
                    let str_val = str_ty.const_zero();
                    let str_val = self
                        .builder
                        .build_insert_value(str_val, buf_ptr, 0, "i2s_p")
                        .unwrap();
                    let str_val = self
                        .builder
                        .build_insert_value(str_val, len, 1, "i2s_l")
                        .unwrap();
                    return Ok(Some(str_val.into_struct_value().into()));
                }

                // Handle sleep_ms(ms) — wraps usleep(ms * 1000)
                if callee == "sleep_ms" {
                    let ms_val = self
                        .compile_expr(&args[0], function)?
                        .unwrap()
                        .into_int_value();
                    let us_val = self
                        .builder
                        .build_int_mul(
                            ms_val,
                            self.context.i32_type().const_int(1000, false),
                            "us",
                        )
                        .unwrap();
                    let usleep_fn = self.get_or_declare_usleep();
                    self.builder
                        .build_call(usleep_fn, &[us_val.into()], "")
                        .unwrap();
                    return Ok(None);
                }

                // Handle sizeof(Type) builtin — compile-time size
                if callee == "sizeof" {
                    // sizeof takes 1 arg — we infer its type and return the size
                    let arg_ty = self.infer_expr_type(&args[0]);
                    let llvm_ty = ny_to_llvm(self.context, &arg_ty);
                    let size = llvm_ty.size_of().unwrap();
                    return Ok(Some(size.into()));
                }

                if let Some((func, _, ret_ty)) = self.functions.get(callee).cloned() {
                    let mut arg_values = Vec::new();
                    for arg in args {
                        let val = self.compile_expr(arg, function)?.unwrap();
                        arg_values.push(val.into());
                    }
                    let call =
                        self.builder.build_call(func, &arg_values, "call").unwrap();

                    if ret_ty == NyType::Unit {
                        Ok(None)
                    } else {
                        Ok(call.try_as_basic_value().basic())
                    }
                } else if let Some((ptr, var_ty)) = self.variables.get(callee).cloned() {
                    // Check if this is a capturing closure
                    if let Some((lambda_fn_name, cap_vars)) =
                        self.closure_captures.get(callee).cloned()
                    {
                        // Call the lambda function directly with captures prepended
                        if let Some((func, _, ret_ty)) =
                            self.functions.get(&lambda_fn_name).cloned()
                        {
                            let mut arg_values: Vec<inkwell::values::BasicMetadataValueEnum> =
                                Vec::new();
                            // Load and pass captured values
                            for (cap_name, cap_ty) in &cap_vars {
                                if let Some((cap_ptr, _)) = self.variables.get(cap_name) {
                                    let cap_llvm = ny_to_llvm(self.context, cap_ty);
                                    let cap_val = self
                                        .builder
                                        .build_load(cap_llvm, *cap_ptr, cap_name)
                                        .unwrap();
                                    arg_values.push(cap_val.into());
                                }
                            }
                            // Pass explicit args
                            for arg in args {
                                let val = self.compile_expr(arg, function)?.unwrap();
                                arg_values.push(val.into());
                            }
                            let call = self
                                .builder
                                .build_call(func, &arg_values, "closure_call")
                                .unwrap();
                            return if ret_ty == NyType::Unit {
                                Ok(None)
                            } else {
                                Ok(call.try_as_basic_value().basic())
                            };
                        }
                    }

                    // Calling a function pointer variable
                    if let NyType::Function { params: param_tys, ret } = &var_ty {
                        let llvm_param_types: Vec<BasicTypeEnum> = param_tys
                            .iter()
                            .map(|t| ny_to_llvm(self.context, t))
                            .collect();
                        let param_meta: Vec<_> =
                            llvm_param_types.iter().map(|t| (*t).into()).collect();
                        let fn_type = match ret.as_ref() {
                            NyType::Unit => {
                                self.context.void_type().fn_type(&param_meta, false)
                            }
                            ty => ny_to_llvm(self.context, ty).fn_type(&param_meta, false),
                        };

                        // Load the function pointer
                        let fn_ptr_val = self
                            .builder
                            .build_load(
                                self.context.ptr_type(AddressSpace::default()),
                                ptr,
                                "fn_ptr",
                            )
                            .unwrap()
                            .into_pointer_value();

                        let mut arg_values = Vec::new();
                        for arg in args {
                            let val = self.compile_expr(arg, function)?.unwrap();
                            arg_values.push(val.into());
                        }

                        let call = self
                            .builder
                            .build_indirect_call(fn_type, fn_ptr_val, &arg_values, "lcall")
                            .unwrap();

                        if **ret == NyType::Unit {
                            Ok(None)
                        } else {
                            Ok(call.try_as_basic_value().basic())
                        }
                    } else {
                        Err(vec![CompileError::type_error(
                            format!("'{}' is not callable", callee),
                            expr.span(),
                        )])
                    }
                } else {
                    Err(vec![CompileError::name_error(
                        format!("undeclared function '{}'", callee),
                        expr.span(),
                    )])
                }
            }

            // ---- If/else ----
            Expr::If {
                condition,
                then_branch,
                else_branch,
                ..
            } => {
                let cond_val = self.compile_expr(condition, function)?.unwrap();
                let cond_int = cond_val.into_int_value();

                let then_bb = self.context.append_basic_block(*function, "then");
                let else_bb = self.context.append_basic_block(*function, "else");
                let merge_bb = self.context.append_basic_block(*function, "merge");

                self.builder
                    .build_conditional_branch(cond_int, then_bb, else_bb)
                    .unwrap();

                // Then branch
                self.builder.position_at_end(then_bb);
                let then_val = self.compile_expr(then_branch, function)?;
                let then_has_terminator = self
                    .builder
                    .get_insert_block()
                    .unwrap()
                    .get_terminator()
                    .is_some();
                let then_end_bb = self.builder.get_insert_block().unwrap();
                if !then_has_terminator {
                    self.builder.build_unconditional_branch(merge_bb).unwrap();
                }

                // Else branch
                self.builder.position_at_end(else_bb);
                let else_val = if let Some(eb) = else_branch {
                    self.compile_expr(eb, function)?
                } else {
                    None
                };
                let else_has_terminator = self
                    .builder
                    .get_insert_block()
                    .unwrap()
                    .get_terminator()
                    .is_some();
                let else_end_bb = self.builder.get_insert_block().unwrap();
                if !else_has_terminator {
                    self.builder.build_unconditional_branch(merge_bb).unwrap();
                }

                self.builder.position_at_end(merge_bb);

                // Build phi if both branches return values and don't have terminators
                if let (Some(tv), Some(ev)) = (&then_val, &else_val) {
                    if !then_has_terminator && !else_has_terminator {
                        let phi = self.builder.build_phi(tv.get_type(), "ifval").unwrap();
                        phi.add_incoming(&[(tv, then_end_bb), (ev, else_end_bb)]);
                        return Ok(Some(phi.as_basic_value()));
                    }
                }

                Ok(None)
            }

            // ---- Block ----
            Expr::Block {
                stmts, tail_expr, ..
            } => {
                for stmt in stmts {
                    self.compile_stmt(stmt, function)?;
                    // If the block has been terminated (return/break/continue), stop
                    if self
                        .builder
                        .get_insert_block()
                        .unwrap()
                        .get_terminator()
                        .is_some()
                    {
                        return Ok(None);
                    }
                }
                if let Some(expr) = tail_expr {
                    self.compile_expr(expr, function)
                } else {
                    Ok(None)
                }
            }

            // ---- Array literal ----
            Expr::ArrayLit { elements, .. } => {
                if elements.is_empty() {
                    // Zero-length array: just return an i32 array of size 0
                    let arr_ty = self.context.i32_type().array_type(0);
                    let alloca = self.builder.build_alloca(arr_ty, "arr").unwrap();
                    return Ok(Some(
                        self.builder.build_load(arr_ty, alloca, "arr_val").unwrap(),
                    ));
                }

                // Compile all elements
                let mut elem_values = Vec::new();
                for elem in elements {
                    let val = self.compile_expr(elem, function)?.unwrap();
                    elem_values.push(val);
                }

                let elem_ty = elem_values[0].get_type();
                let arr_size = elem_values.len() as u32;
                let arr_ty = elem_ty.array_type(arr_size);
                let alloca = self.builder.build_alloca(arr_ty, "arr").unwrap();

                let zero = self.context.i64_type().const_zero();
                for (i, val) in elem_values.iter().enumerate() {
                    let idx = self.context.i64_type().const_int(i as u64, false);
                    let gep = unsafe {
                        self.builder
                            .build_in_bounds_gep(arr_ty, alloca, &[zero, idx], "arr_elem_ptr")
                            .unwrap()
                    };
                    self.builder.build_store(gep, *val).unwrap();
                }

                // Load the whole array value
                let arr_val = self.builder.build_load(arr_ty, alloca, "arr_val").unwrap();
                Ok(Some(arr_val))
            }

            // ---- Index access (read) ----
            Expr::Index { object, index, .. } => {
                let obj_ty = self.infer_expr_type(object);
                let idx_val = self
                    .compile_expr(index, function)?
                    .unwrap()
                    .into_int_value();

                match &obj_ty {
                    NyType::Array { elem, size } => {
                        let elem_llvm = ny_to_llvm(self.context, elem);
                        let arr_llvm = elem_llvm.array_type(*size as u32);

                        // Get pointer to the array (we need the alloca, not the loaded value)
                        let arr_ptr = self.compile_expr_as_ptr(object, function)?;

                        // Bounds check
                        let arr_len = self.context.i64_type().const_int(*size as u64, false);
                        let idx_i64 = self
                            .builder
                            .build_int_z_extend_or_bit_cast(
                                idx_val,
                                self.context.i64_type(),
                                "idx_ext",
                            )
                            .unwrap();
                        self.build_bounds_check(idx_i64, arr_len, function);

                        let zero = self.context.i64_type().const_zero();
                        let gep = unsafe {
                            self.builder
                                .build_in_bounds_gep(arr_llvm, arr_ptr, &[zero, idx_i64], "idx_ptr")
                                .unwrap()
                        };

                        let val = self.builder.build_load(elem_llvm, gep, "idx_val").unwrap();
                        Ok(Some(val))
                    }
                    NyType::Slice(elem) => {
                        // Slice indexing: extract ptr from {ptr, len}, GEP, load
                        let elem_llvm = ny_to_llvm(self.context, elem);
                        let slice_val = self
                            .compile_expr(object, function)?
                            .unwrap()
                            .into_struct_value();
                        let ptr = self
                            .builder
                            .build_extract_value(slice_val, 0, "slice_ptr")
                            .unwrap()
                            .into_pointer_value();
                        let len = self
                            .builder
                            .build_extract_value(slice_val, 1, "slice_len")
                            .unwrap()
                            .into_int_value();

                        let idx_i64 = self
                            .builder
                            .build_int_z_extend_or_bit_cast(
                                idx_val,
                                self.context.i64_type(),
                                "idx_ext",
                            )
                            .unwrap();
                        self.build_bounds_check(idx_i64, len, function);

                        let gep = unsafe {
                            self.builder
                                .build_in_bounds_gep(elem_llvm, ptr, &[idx_i64], "slice_idx_ptr")
                                .unwrap()
                        };
                        let val =
                            self.builder.build_load(elem_llvm, gep, "slice_idx_val").unwrap();
                        Ok(Some(val))
                    }
                    _ => Err(vec![CompileError::type_error(
                        "cannot index non-array type".to_string(),
                        object.span(),
                    )]),
                }
            }

            // ---- Struct initialization ----
            Expr::StructInit { name, fields, .. } => {
                let struct_fields = self.struct_types.get(name).cloned().unwrap_or_default();
                let struct_ty = self.get_or_create_llvm_struct_type(name, &struct_fields);
                let alloca = self
                    .builder
                    .build_alloca(struct_ty, &format!("{}_init", name))
                    .unwrap();

                // Store each field value at the correct index
                for (field_name, field_expr) in fields {
                    let field_idx = struct_fields
                        .iter()
                        .position(|(n, _)| n == field_name)
                        .unwrap_or(0) as u32;
                    let val = self.compile_expr(field_expr, function)?.unwrap();
                    let field_ptr = self
                        .builder
                        .build_struct_gep(
                            struct_ty,
                            alloca,
                            field_idx,
                            &format!("{}_ptr", field_name),
                        )
                        .unwrap();
                    self.builder.build_store(field_ptr, val).unwrap();
                }

                // Load the whole struct value
                let struct_val = self
                    .builder
                    .build_load(struct_ty, alloca, &format!("{}_val", name))
                    .unwrap();
                Ok(Some(struct_val))
            }

            // ---- Field access ----
            Expr::FieldAccess { object, field, .. } => {
                let obj_ty = self.infer_expr_type(object);

                // Handle auto-deref for pointers to structs
                let (struct_name, struct_fields, is_pointer) = match &obj_ty {
                    NyType::Struct { name, fields } => (name.clone(), fields.clone(), false),
                    NyType::Pointer(inner) => match inner.as_ref() {
                        NyType::Struct { name, fields } => (name.clone(), fields.clone(), true),
                        _ => {
                            return Err(vec![CompileError::type_error(
                                "field access on non-struct pointer".to_string(),
                                object.span(),
                            )]);
                        }
                    },
                    _ => {
                        return Err(vec![CompileError::type_error(
                            format!("field access on non-struct type '{}'", obj_ty),
                            object.span(),
                        )]);
                    }
                };

                let field_idx = struct_fields
                    .iter()
                    .position(|(n, _)| n == field)
                    .unwrap_or(0) as u32;
                let field_ty_ny = struct_fields
                    .get(field_idx as usize)
                    .map(|(_, t)| t.clone())
                    .unwrap_or(NyType::I32);
                let field_ty_llvm = ny_to_llvm(self.context, &field_ty_ny);

                let struct_llvm_ty =
                    self.get_or_create_llvm_struct_type(&struct_name, &struct_fields);

                let struct_ptr = if is_pointer {
                    // Auto-deref: the object is a pointer to the struct, load the pointer
                    let ptr_val = self.compile_expr(object, function)?.unwrap();
                    ptr_val.into_pointer_value()
                } else {
                    // The object is a struct value; get its alloca pointer
                    self.compile_expr_as_ptr(object, function)?
                };

                let field_ptr = self
                    .builder
                    .build_struct_gep(
                        struct_llvm_ty,
                        struct_ptr,
                        field_idx,
                        &format!("{}_field", field),
                    )
                    .unwrap();

                let val = self
                    .builder
                    .build_load(field_ty_llvm, field_ptr, field)
                    .unwrap();
                Ok(Some(val))
            }

            // ---- Address-of (&expr) ----
            Expr::AddrOf { operand, .. } => {
                let ptr = self.compile_expr_as_ptr(operand, function)?;
                Ok(Some(ptr.into()))
            }

            // ---- Dereference (*expr) ----
            Expr::Deref { operand, .. } => {
                let ptr_val = self.compile_expr(operand, function)?.unwrap();
                let ptr = ptr_val.into_pointer_value();
                let pointee_ty = self.infer_expr_type(expr);
                let llvm_ty = ny_to_llvm(self.context, &pointee_ty);
                let val = self.builder.build_load(llvm_ty, ptr, "deref").unwrap();
                Ok(Some(val))
            }

            // ---- Method call ----
            Expr::MethodCall {
                object,
                method,
                args,
                ..
            } => {
                let obj_ty = self.infer_expr_type(object);

                // Handle built-in Vec methods
                if let NyType::Vec(elem_ty) = &obj_ty {
                    let elem_llvm = ny_to_llvm(self.context, elem_ty);
                    let vec_struct_ty = ny_to_llvm(self.context, &obj_ty).into_struct_type();

                    match method.as_str() {
                        "len" => {
                            let obj_val = self.compile_expr(object, function)?.unwrap();
                            let sv = obj_val.into_struct_value();
                            let len = self
                                .builder
                                .build_extract_value(sv, 1, "vec_len")
                                .unwrap();
                            return Ok(Some(len));
                        }
                        "get" => {
                            let obj_val = self.compile_expr(object, function)?.unwrap();
                            let sv = obj_val.into_struct_value();
                            let data_ptr = self
                                .builder
                                .build_extract_value(sv, 0, "vec_data")
                                .unwrap()
                                .into_pointer_value();
                            let len = self
                                .builder
                                .build_extract_value(sv, 1, "vec_len_check")
                                .unwrap()
                                .into_int_value();
                            let idx = self
                                .compile_expr(&args[0], function)?
                                .unwrap()
                                .into_int_value();
                            let idx_i64 = self
                                .builder
                                .build_int_z_extend_or_bit_cast(
                                    idx,
                                    self.context.i64_type(),
                                    "idx64",
                                )
                                .unwrap();
                            // Bounds check
                            self.build_bounds_check(idx_i64, len, function);
                            let gep = unsafe {
                                self.builder
                                    .build_in_bounds_gep(
                                        elem_llvm,
                                        data_ptr,
                                        &[idx_i64],
                                        "vec_elem_ptr",
                                    )
                                    .unwrap()
                            };
                            let val = self
                                .builder
                                .build_load(elem_llvm, gep, "vec_elem")
                                .unwrap();
                            return Ok(Some(val));
                        }
                        "push" => {
                            // Need the alloca pointer to mutate the vec
                            let vec_ptr = self.compile_expr_as_ptr(object, function)?;
                            let val = self.compile_expr(&args[0], function)?.unwrap();

                            // Load current data, len, cap
                            let data_gep = self
                                .builder
                                .build_struct_gep(vec_struct_ty, vec_ptr, 0, "data_gep")
                                .unwrap();
                            let len_gep = self
                                .builder
                                .build_struct_gep(vec_struct_ty, vec_ptr, 1, "len_gep")
                                .unwrap();
                            let cap_gep = self
                                .builder
                                .build_struct_gep(vec_struct_ty, vec_ptr, 2, "cap_gep")
                                .unwrap();

                            let data_ptr = self
                                .builder
                                .build_load(
                                    self.context.ptr_type(AddressSpace::default()),
                                    data_gep,
                                    "data",
                                )
                                .unwrap()
                                .into_pointer_value();
                            let len = self
                                .builder
                                .build_load(self.context.i64_type(), len_gep, "len")
                                .unwrap()
                                .into_int_value();
                            let cap = self
                                .builder
                                .build_load(self.context.i64_type(), cap_gep, "cap")
                                .unwrap()
                                .into_int_value();

                            // Check if we need to grow: if len >= cap, realloc
                            let needs_grow = self
                                .builder
                                .build_int_compare(IntPredicate::UGE, len, cap, "needs_grow")
                                .unwrap();

                            let grow_bb =
                                self.context.append_basic_block(*function, "vec_grow");
                            let push_bb =
                                self.context.append_basic_block(*function, "vec_push");

                            self.builder
                                .build_conditional_branch(needs_grow, grow_bb, push_bb)
                                .unwrap();

                            // Grow: double capacity, realloc
                            self.builder.position_at_end(grow_bb);
                            let new_cap = self
                                .builder
                                .build_int_mul(
                                    cap,
                                    self.context.i64_type().const_int(2, false),
                                    "new_cap",
                                )
                                .unwrap();
                            let elem_size = elem_llvm.size_of().unwrap();
                            let new_size = self
                                .builder
                                .build_int_mul(new_cap, elem_size, "new_size")
                                .unwrap();
                            let realloc_fn = self.get_or_declare_realloc();
                            let new_data = self
                                .builder
                                .build_call(
                                    realloc_fn,
                                    &[data_ptr.into(), new_size.into()],
                                    "new_data",
                                )
                                .unwrap()
                                .try_as_basic_value()
                                .basic()
                                .unwrap()
                                .into_pointer_value();
                            self.builder.build_store(data_gep, new_data).unwrap();
                            self.builder.build_store(cap_gep, new_cap).unwrap();
                            self.builder
                                .build_unconditional_branch(push_bb)
                                .unwrap();

                            // Push: store value at data[len], increment len
                            self.builder.position_at_end(push_bb);
                            // Re-load data ptr (may have changed from realloc)
                            let current_data = self
                                .builder
                                .build_load(
                                    self.context.ptr_type(AddressSpace::default()),
                                    data_gep,
                                    "cur_data",
                                )
                                .unwrap()
                                .into_pointer_value();
                            let current_len = self
                                .builder
                                .build_load(self.context.i64_type(), len_gep, "cur_len")
                                .unwrap()
                                .into_int_value();

                            let elem_ptr = unsafe {
                                self.builder
                                    .build_in_bounds_gep(
                                        elem_llvm,
                                        current_data,
                                        &[current_len],
                                        "push_ptr",
                                    )
                                    .unwrap()
                            };
                            self.builder.build_store(elem_ptr, val).unwrap();

                            // len += 1
                            let new_len = self
                                .builder
                                .build_int_add(
                                    current_len,
                                    self.context.i64_type().const_int(1, false),
                                    "new_len",
                                )
                                .unwrap();
                            self.builder.build_store(len_gep, new_len).unwrap();

                            return Ok(None);
                        }
                        _ => {}
                    }
                }

                // Handle built-in slice methods
                if let NyType::Slice(_) = &obj_ty {
                    let obj_val = self.compile_expr(object, function)?.unwrap();
                    let slice_val = obj_val.into_struct_value();
                    if method.as_str() == "len" {
                        let len_val = self
                            .builder
                            .build_extract_value(slice_val, 1, "slice_len")
                            .unwrap();
                        return Ok(Some(len_val));
                    }
                }

                // Handle built-in string methods
                if obj_ty == NyType::Str {
                    let obj_val = self.compile_expr(object, function)?.unwrap();
                    let str_val = obj_val.into_struct_value();

                    match method.as_str() {
                        "len" => {
                            // Extract length field (index 1) from {ptr, len}
                            let len_val = self
                                .builder
                                .build_extract_value(str_val, 1, "str_len")
                                .unwrap();
                            return Ok(Some(len_val));
                        }
                        "substr" => {
                            // substr(start, end) -> {new_ptr, new_len}
                            let start_val = self
                                .compile_expr(&args[0], function)?
                                .unwrap()
                                .into_int_value();
                            let end_val = self
                                .compile_expr(&args[1], function)?
                                .unwrap()
                                .into_int_value();

                            let ptr = self
                                .builder
                                .build_extract_value(str_val, 0, "str_ptr")
                                .unwrap()
                                .into_pointer_value();

                            // Cast start to i64 for GEP
                            let start_i64 = self
                                .builder
                                .build_int_z_extend_or_bit_cast(
                                    start_val,
                                    self.context.i64_type(),
                                    "start_ext",
                                )
                                .unwrap();
                            let end_i64 = self
                                .builder
                                .build_int_z_extend_or_bit_cast(
                                    end_val,
                                    self.context.i64_type(),
                                    "end_ext",
                                )
                                .unwrap();

                            // new_ptr = GEP(ptr, start)
                            let i8_ty = self.context.i8_type();
                            let new_ptr = unsafe {
                                self.builder
                                    .build_in_bounds_gep(i8_ty, ptr, &[start_i64], "substr_ptr")
                                    .unwrap()
                            };

                            // new_len = end - start
                            let new_len = self
                                .builder
                                .build_int_sub(end_i64, start_i64, "substr_len")
                                .unwrap();

                            // Build {new_ptr, new_len}
                            let str_ty = str_type(self.context);
                            let result = str_ty.const_zero();
                            let result = self
                                .builder
                                .build_insert_value(result, new_ptr, 0, "sub_ptr")
                                .unwrap();
                            let result = self
                                .builder
                                .build_insert_value(result, new_len, 1, "sub_len")
                                .unwrap();
                            return Ok(Some(result.into_struct_value().into()));
                        }
                        _ => {}
                    }
                }

                // Compile the object as the first argument (pass by value or pointer)
                let obj_val = self.compile_expr(object, function)?.unwrap();
                let callee = method.clone();

                if let Some((func, _, ret_ty)) = self.functions.get(&callee).cloned() {
                    let mut arg_values = vec![obj_val.into()];
                    for arg in args {
                        let val = self.compile_expr(arg, function)?.unwrap();
                        arg_values.push(val.into());
                    }
                    let call = self
                        .builder
                        .build_call(func, &arg_values, "method_call")
                        .unwrap();
                    if ret_ty == NyType::Unit {
                        Ok(None)
                    } else {
                        Ok(call.try_as_basic_value().basic())
                    }
                } else {
                    // Try struct_name.method_name convention
                    let struct_name = match &obj_ty {
                        NyType::Struct { name, .. } => name.clone(),
                        NyType::Pointer(inner) => match inner.as_ref() {
                            NyType::Struct { name, .. } => name.clone(),
                            _ => String::new(),
                        },
                        _ => String::new(),
                    };
                    let qualified_name = format!("{}_{}", struct_name, method);
                    if let Some((func, _, ret_ty)) = self.functions.get(&qualified_name).cloned() {
                        let mut arg_values = vec![obj_val.into()];
                        for arg in args {
                            let val = self.compile_expr(arg, function)?.unwrap();
                            arg_values.push(val.into());
                        }
                        let call = self
                            .builder
                            .build_call(func, &arg_values, "method_call")
                            .unwrap();
                        if ret_ty == NyType::Unit {
                            Ok(None)
                        } else {
                            Ok(call.try_as_basic_value().basic())
                        }
                    } else {
                        Err(vec![CompileError::name_error(
                            format!("undeclared method '{}'", method),
                            expr.span(),
                        )])
                    }
                }
            }

            // ---- Type cast (expr as T) ----
            Expr::Cast {
                expr: inner_expr,
                target_type,
                ..
            } => {
                let val = self.compile_expr(inner_expr, function)?.unwrap();
                let source_ty = self.infer_expr_type(inner_expr);
                let target_ty = self.resolve_type_annotation(target_type);
                let target_llvm = ny_to_llvm(self.context, &target_ty);

                if source_ty == target_ty {
                    return Ok(Some(val)); // no-op
                }

                let result = self.compile_cast(val, &source_ty, &target_ty, target_llvm)?;
                Ok(Some(result))
            }

            // ---- Enum variant ----
            // ---- Try (?) operator ----
            Expr::Try { operand, .. } => {
                let subject_raw = self.compile_expr(operand, function)?.unwrap();
                let subject_ty = self.infer_expr_type(operand);

                let enum_name = match &subject_ty {
                    NyType::Enum { name, .. } => name.clone(),
                    _ => {
                        return Err(vec![CompileError::type_error(
                            "? requires enum type".to_string(),
                            expr.span(),
                        )]);
                    }
                };

                if self.enum_has_payload(&enum_name) {
                    let enum_ty = self.enum_struct_type(&enum_name);
                    let alloca = self
                        .builder
                        .build_alloca(enum_ty, "try_subject")
                        .unwrap();
                    self.builder.build_store(alloca, subject_raw).unwrap();

                    // Extract tag
                    let tag_ptr = self
                        .builder
                        .build_struct_gep(enum_ty, alloca, 0, "try_tag_ptr")
                        .unwrap();
                    let tag = self
                        .builder
                        .build_load(self.context.i32_type(), tag_ptr, "try_tag")
                        .unwrap()
                        .into_int_value();

                    let ok_bb = self
                        .context
                        .append_basic_block(*function, "try_ok");
                    let err_bb = self
                        .context
                        .append_basic_block(*function, "try_err");

                    // tag == 0 means first variant (Ok)
                    let zero = self.context.i32_type().const_zero();
                    let is_ok = self
                        .builder
                        .build_int_compare(IntPredicate::EQ, tag, zero, "is_ok")
                        .unwrap();
                    self.builder
                        .build_conditional_branch(is_ok, ok_bb, err_bb)
                        .unwrap();

                    // Err path: return the whole enum from the current function
                    self.builder.position_at_end(err_bb);
                    // Emit defers before early return
                    let defers: Vec<(Expr, FunctionValue<'ctx>)> =
                        self.defer_stack.iter().rev().cloned().collect();
                    for (defer_body, defer_fn) in &defers {
                        self.compile_expr(defer_body, defer_fn)?;
                    }
                    self.builder.build_return(Some(&subject_raw)).unwrap();

                    // Ok path: extract the first payload field
                    self.builder.position_at_end(ok_bb);
                    let payload_ptr = self
                        .builder
                        .build_struct_gep(enum_ty, alloca, 1, "try_payload_ptr")
                        .unwrap();
                    let result_ty = self.infer_expr_type(expr);
                    let payload_llvm = ny_to_llvm(self.context, &result_ty);
                    let payload_val = self
                        .builder
                        .build_load(payload_llvm, payload_ptr, "try_payload")
                        .unwrap();
                    Ok(Some(payload_val))
                } else {
                    // Simple enum — can't use ? on it meaningfully
                    Ok(Some(subject_raw))
                }
            }

            // ---- Lambda (with optional captures via lambda lifting) ----
            Expr::Lambda {
                params,
                return_type,
                body,
                ..
            } => {
                static LAMBDA_COUNTER: std::sync::atomic::AtomicUsize =
                    std::sync::atomic::AtomicUsize::new(0);
                let id = LAMBDA_COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                let lambda_name = format!("__lambda_{}", id);

                let ret_ty = self.resolve_type_annotation(return_type);
                let param_types: Vec<NyType> = params
                    .iter()
                    .map(|p| self.resolve_type_annotation(&p.ty))
                    .collect();
                let param_names: Vec<String> = params.iter().map(|p| p.name.clone()).collect();

                // Find captured variables: scan body for Ident references
                // that are not in the lambda's param list but exist in outer scope
                let mut captures: Vec<(String, NyType)> = Vec::new();
                let mut capture_values: Vec<BasicValueEnum<'ctx>> = Vec::new();
                {
                    let free_vars = find_free_vars(body, &param_names);
                    for var_name in &free_vars {
                        if let Some((ptr, ty)) = self.variables.get(var_name) {
                            let llvm_ty = ny_to_llvm(self.context, ty);
                            let val = self
                                .builder
                                .build_load(llvm_ty, *ptr, &format!("cap_{}", var_name))
                                .unwrap();
                            captures.push((var_name.clone(), ty.clone()));
                            capture_values.push(val);
                        }
                    }
                }

                // Build the function with captures as prefix params
                let mut all_param_types: Vec<NyType> = Vec::new();
                for (_, ty) in &captures {
                    all_param_types.push(ty.clone());
                }
                all_param_types.extend(param_types.clone());

                let llvm_param_types: Vec<BasicTypeEnum> = all_param_types
                    .iter()
                    .map(|t| ny_to_llvm(self.context, t))
                    .collect();
                let param_meta: Vec<_> = llvm_param_types.iter().map(|t| (*t).into()).collect();

                let fn_type = match &ret_ty {
                    NyType::Unit => self.context.void_type().fn_type(&param_meta, false),
                    ty => ny_to_llvm(self.context, ty).fn_type(&param_meta, false),
                };

                let lambda_fn = self.module.add_function(&lambda_name, fn_type, None);
                self.functions.insert(
                    lambda_name.clone(),
                    (lambda_fn, all_param_types.clone(), ret_ty),
                );

                // Save state
                let outer_vars = self.variables.clone();
                self.variables.clear();
                let current_bb = self.builder.get_insert_block().unwrap();

                // Build lambda body
                let entry = self.context.append_basic_block(lambda_fn, "entry");
                self.builder.position_at_end(entry);

                // Set up captured variables as params
                for (i, (name, ty)) in captures.iter().enumerate() {
                    let llvm_ty = ny_to_llvm(self.context, ty);
                    let alloca = self.builder.build_alloca(llvm_ty, name).unwrap();
                    self.builder
                        .build_store(alloca, lambda_fn.get_nth_param(i as u32).unwrap())
                        .unwrap();
                    self.variables.insert(name.clone(), (alloca, ty.clone()));
                }

                // Set up explicit params
                let cap_count = captures.len();
                for (i, param) in params.iter().enumerate() {
                    let ty = &param_types[i];
                    let llvm_ty = ny_to_llvm(self.context, ty);
                    let alloca = self.builder.build_alloca(llvm_ty, &param.name).unwrap();
                    self.builder
                        .build_store(
                            alloca,
                            lambda_fn.get_nth_param((cap_count + i) as u32).unwrap(),
                        )
                        .unwrap();
                    self.variables
                        .insert(param.name.clone(), (alloca, ty.clone()));
                }

                self.compile_expr(body, &lambda_fn)?;

                let lambda_end_bb = self.builder.get_insert_block().unwrap();
                if lambda_end_bb.get_terminator().is_none() {
                    self.builder.build_return(None).unwrap();
                }

                // Restore state
                self.variables = outer_vars;
                self.builder.position_at_end(current_bb);

                let fn_ptr = lambda_fn.as_global_value().as_pointer_value();

                if !captures.is_empty() {
                    // Create dedicated allocas to store captured values by-value
                    // These persist after the lambda is created, even if the original
                    // variable is reassigned.
                    let mut capture_alloca_names: Vec<(String, NyType)> = Vec::new();
                    for (i, ((cap_name, cap_ty), cap_val)) in
                        captures.iter().zip(capture_values.iter()).enumerate()
                    {
                        let alloca_name = format!("__cl{}_{}", id, cap_name);
                        let llvm_ty = ny_to_llvm(self.context, cap_ty);
                        let alloca = self
                            .builder
                            .build_alloca(llvm_ty, &alloca_name)
                            .unwrap();
                        self.builder.build_store(alloca, *cap_val).unwrap();
                        self.variables
                            .insert(alloca_name.clone(), (alloca, cap_ty.clone()));
                        capture_alloca_names.push((alloca_name, cap_ty.clone()));
                    }

                    self.closure_captures.insert(
                        lambda_name.clone(),
                        (lambda_name, capture_alloca_names),
                    );
                }

                Ok(Some(fn_ptr.into()))
            }

            // ---- Range index (arr[start..end] → slice {ptr, len}) ----
            Expr::RangeIndex {
                object,
                start,
                end,
                ..
            } => {
                let obj_ty = self.infer_expr_type(object);
                let start_val = self
                    .compile_expr(start, function)?
                    .unwrap()
                    .into_int_value();
                let end_val = self
                    .compile_expr(end, function)?
                    .unwrap()
                    .into_int_value();

                let start_i64 = self
                    .builder
                    .build_int_z_extend_or_bit_cast(
                        start_val,
                        self.context.i64_type(),
                        "start_ext",
                    )
                    .unwrap();
                let end_i64 = self
                    .builder
                    .build_int_z_extend_or_bit_cast(
                        end_val,
                        self.context.i64_type(),
                        "end_ext",
                    )
                    .unwrap();

                match &obj_ty {
                    NyType::Array { elem, size } => {
                        let elem_llvm = ny_to_llvm(self.context, elem);
                        let arr_llvm = elem_llvm.array_type(*size as u32);
                        let arr_ptr = self.compile_expr_as_ptr(object, function)?;
                        let zero = self.context.i64_type().const_zero();
                        let gep = unsafe {
                            self.builder
                                .build_in_bounds_gep(
                                    arr_llvm,
                                    arr_ptr,
                                    &[zero, start_i64],
                                    "slice_ptr",
                                )
                                .unwrap()
                        };
                        let len = self
                            .builder
                            .build_int_sub(end_i64, start_i64, "slice_len")
                            .unwrap();
                        // Build {ptr, len} slice struct
                        let slice_ty = self.context.struct_type(
                            &[
                                self.context.ptr_type(AddressSpace::default()).into(),
                                self.context.i64_type().into(),
                            ],
                            false,
                        );
                        let slice_val = slice_ty.const_zero();
                        let slice_val = self
                            .builder
                            .build_insert_value(slice_val, gep, 0, "slice_p")
                            .unwrap();
                        let slice_val = self
                            .builder
                            .build_insert_value(slice_val, len, 1, "slice_l")
                            .unwrap();
                        Ok(Some(slice_val.into_struct_value().into()))
                    }
                    _ => Err(vec![CompileError::type_error(
                        "range index on non-array type".to_string(),
                        object.span(),
                    )]),
                }
            }

            Expr::EnumVariant {
                enum_name,
                variant,
                args,
                ..
            } => {
                if let Some(variant_defs) = self.enum_variants.get(enum_name).cloned() {
                    let idx = variant_defs
                        .iter()
                        .position(|(name, _)| name == variant)
                        .unwrap_or(0);
                    let tag_val = self.context.i32_type().const_int(idx as u64, false);

                    if !self.enum_has_payload(enum_name) || args.is_empty() {
                        // Simple enum — just return the discriminant
                        Ok(Some(tag_val.into()))
                    } else {
                        // Data-carrying enum — build { tag, payload... } struct
                        let enum_ty = self.enum_struct_type(enum_name);
                        let alloca = self
                            .builder
                            .build_alloca(enum_ty, "enum_val")
                            .unwrap();

                        // Store tag
                        let tag_ptr = self
                            .builder
                            .build_struct_gep(enum_ty, alloca, 0, "tag_ptr")
                            .unwrap();
                        self.builder.build_store(tag_ptr, tag_val).unwrap();

                        // Store payload fields
                        for (i, arg) in args.iter().enumerate() {
                            let val = self.compile_expr(arg, function)?.unwrap();
                            let field_ptr = self
                                .builder
                                .build_struct_gep(
                                    enum_ty,
                                    alloca,
                                    (i + 1) as u32,
                                    &format!("payload_{}", i),
                                )
                                .unwrap();
                            self.builder.build_store(field_ptr, val).unwrap();
                        }

                        let result = self
                            .builder
                            .build_load(enum_ty, alloca, "enum_loaded")
                            .unwrap();
                        Ok(Some(result))
                    }
                } else {
                    Err(vec![CompileError::type_error(
                        format!("unknown enum '{}'", enum_name),
                        expr.span(),
                    )])
                }
            }

            // ---- Match expression ----
            Expr::Match { subject, arms, .. } => {
                let subject_raw = self.compile_expr(subject, function)?.unwrap();
                let subject_ty = self.infer_expr_type(subject);
                let result_ty = self.infer_expr_type(expr);
                let has_result = result_ty != NyType::Unit;

                // Determine if subject is a data-carrying enum
                let is_tagged_union = match &subject_ty {
                    NyType::Enum { name, .. } => self.enum_has_payload(name),
                    _ => false,
                };

                // Extract the tag (discriminant) for the switch
                let (subject_val, subject_ptr) = if is_tagged_union {
                    let enum_name = match &subject_ty {
                        NyType::Enum { name, .. } => name.clone(),
                        _ => unreachable!(),
                    };
                    let enum_ty = self.enum_struct_type(&enum_name);
                    // Store subject to alloca so we can GEP into it
                    let alloca = self
                        .builder
                        .build_alloca(enum_ty, "match_subject")
                        .unwrap();
                    self.builder.build_store(alloca, subject_raw).unwrap();
                    // Extract tag
                    let tag_ptr = self
                        .builder
                        .build_struct_gep(enum_ty, alloca, 0, "tag_ptr")
                        .unwrap();
                    let tag = self
                        .builder
                        .build_load(self.context.i32_type(), tag_ptr, "tag")
                        .unwrap()
                        .into_int_value();
                    (tag, Some(alloca))
                } else {
                    (subject_raw.into_int_value(), None)
                };

                let merge_bb = self.context.append_basic_block(*function, "match_merge");

                let wildcard_idx = arms
                    .iter()
                    .position(|arm| matches!(arm.pattern, Pattern::Wildcard(_)));

                let arm_bbs: Vec<BasicBlock> = arms
                    .iter()
                    .enumerate()
                    .map(|(i, _)| {
                        self.context
                            .append_basic_block(*function, &format!("match_arm_{}", i))
                    })
                    .collect();

                let switch_bb = self.builder.get_insert_block().unwrap();

                let default_bb = if let Some(wi) = wildcard_idx {
                    arm_bbs[wi]
                } else {
                    let unreachable_bb = self
                        .context
                        .append_basic_block(*function, "match_unreachable");
                    self.builder.position_at_end(unreachable_bb);
                    self.builder.build_unreachable().unwrap();
                    unreachable_bb
                };

                let mut cases: Vec<(inkwell::values::IntValue<'ctx>, BasicBlock<'ctx>)> =
                    Vec::new();
                for (i, arm) in arms.iter().enumerate() {
                    match &arm.pattern {
                        Pattern::EnumVariant {
                            enum_name, variant, ..
                        } => {
                            if let Some(variant_defs) = self.enum_variants.get(enum_name) {
                                let idx = variant_defs
                                    .iter()
                                    .position(|(name, _)| name == variant)
                                    .unwrap_or(0);
                                let const_val =
                                    self.context.i32_type().const_int(idx as u64, false);
                                cases.push((const_val, arm_bbs[i]));
                            }
                        }
                        Pattern::IntLit(n, _) => {
                            let const_val = self.context.i32_type().const_int(*n as u64, true);
                            cases.push((const_val, arm_bbs[i]));
                        }
                        Pattern::Wildcard(_) => {}
                    }
                }

                self.builder.position_at_end(switch_bb);
                self.builder
                    .build_switch(subject_val, default_bb, &cases)
                    .unwrap();

                // Compile each arm body, extracting payload bindings for tagged unions
                let mut incoming: Vec<(BasicValueEnum<'ctx>, BasicBlock<'ctx>)> = Vec::new();
                for (i, arm) in arms.iter().enumerate() {
                    self.builder.position_at_end(arm_bbs[i]);

                    // Extract payload bindings for data-carrying enum patterns
                    if let Pattern::EnumVariant {
                        enum_name,
                        bindings,
                        variant,
                        ..
                    } = &arm.pattern
                    {
                        if !bindings.is_empty() {
                            if let Some(alloca) = subject_ptr {
                                let enum_ty = self.enum_struct_type(enum_name);
                                // Find the variant's payload types
                                let payload_types = self
                                    .enum_variants
                                    .get(enum_name)
                                    .and_then(|vs| {
                                        vs.iter()
                                            .find(|(n, _)| n == variant)
                                            .map(|(_, p)| p.clone())
                                    })
                                    .unwrap_or_default();

                                for (j, binding_name) in bindings.iter().enumerate() {
                                    let field_idx = (j + 1) as u32; // skip tag
                                    let field_ptr = self
                                        .builder
                                        .build_struct_gep(
                                            enum_ty,
                                            alloca,
                                            field_idx,
                                            &format!("bind_{}", binding_name),
                                        )
                                        .unwrap();
                                    let payload_ny_ty = payload_types
                                        .get(j)
                                        .cloned()
                                        .unwrap_or(NyType::I32);
                                    let payload_llvm_ty =
                                        ny_to_llvm(self.context, &payload_ny_ty);
                                    let val = self
                                        .builder
                                        .build_load(
                                            payload_llvm_ty,
                                            field_ptr,
                                            binding_name,
                                        )
                                        .unwrap();
                                    // Declare binding as a variable
                                    let bind_alloca = self
                                        .builder
                                        .build_alloca(payload_llvm_ty, binding_name)
                                        .unwrap();
                                    self.builder.build_store(bind_alloca, val).unwrap();
                                    self.variables.insert(
                                        binding_name.clone(),
                                        (bind_alloca, payload_ny_ty),
                                    );
                                }
                            }
                        }
                    }

                    let arm_val = self.compile_expr(&arm.body, function)?;
                    let arm_end_bb = self.builder.get_insert_block().unwrap();
                    let has_terminator = arm_end_bb.get_terminator().is_some();
                    if !has_terminator {
                        if has_result {
                            if let Some(v) = arm_val {
                                incoming.push((v, arm_end_bb));
                            }
                        }
                        self.builder.build_unconditional_branch(merge_bb).unwrap();
                    }
                }

                self.builder.position_at_end(merge_bb);

                if has_result && !incoming.is_empty() {
                    let llvm_ty = ny_to_llvm(self.context, &result_ty);
                    let phi = self.builder.build_phi(llvm_ty, "match_val").unwrap();
                    let refs: Vec<(&dyn inkwell::values::BasicValue, BasicBlock)> = incoming
                        .iter()
                        .map(|(v, bb)| (v as &dyn inkwell::values::BasicValue, *bb))
                        .collect();
                    phi.add_incoming(&refs);
                    Ok(Some(phi.as_basic_value()))
                } else {
                    Ok(None)
                }
            }

            // ---- Tuple literal ----
            Expr::TupleLit { elements, .. } => {
                let mut elem_values = Vec::new();
                for elem in elements {
                    let val = self.compile_expr(elem, function)?.unwrap();
                    elem_values.push(val);
                }

                let elem_types: Vec<BasicTypeEnum> =
                    elem_values.iter().map(|v| v.get_type()).collect();
                let tuple_ty = self.context.struct_type(&elem_types, false);
                let alloca = self.builder.build_alloca(tuple_ty, "tuple").unwrap();

                for (i, val) in elem_values.iter().enumerate() {
                    let field_ptr = self
                        .builder
                        .build_struct_gep(tuple_ty, alloca, i as u32, &format!("tuple_field_{}", i))
                        .unwrap();
                    self.builder.build_store(field_ptr, *val).unwrap();
                }

                let tuple_val = self
                    .builder
                    .build_load(tuple_ty, alloca, "tuple_val")
                    .unwrap();
                Ok(Some(tuple_val))
            }

            // ---- Tuple index ----
            Expr::TupleIndex { object, index, .. } => {
                let obj_val = self.compile_expr(object, function)?.unwrap();
                let struct_val = obj_val.into_struct_value();
                let extracted = self
                    .builder
                    .build_extract_value(struct_val, *index as u32, "tuple_idx")
                    .unwrap();
                Ok(Some(extracted))
            }
        }
    }

    fn compile_cast(
        &self,
        val: BasicValueEnum<'ctx>,
        source_ty: &NyType,
        target_ty: &NyType,
        target_llvm: BasicTypeEnum<'ctx>,
    ) -> Result<BasicValueEnum<'ctx>, Vec<CompileError>> {
        match (source_ty, target_ty) {
            // int → int
            (s, t) if s.is_integer() && t.is_integer() => {
                let src_bits = self.int_bit_width(s);
                let tgt_bits = self.int_bit_width(t);
                let int_val = val.into_int_value();
                let tgt_int_ty = target_llvm.into_int_type();
                if tgt_bits > src_bits {
                    if s.is_signed() {
                        Ok(self
                            .builder
                            .build_int_s_extend(int_val, tgt_int_ty, "sext")
                            .unwrap()
                            .into())
                    } else {
                        Ok(self
                            .builder
                            .build_int_z_extend(int_val, tgt_int_ty, "zext")
                            .unwrap()
                            .into())
                    }
                } else if tgt_bits < src_bits {
                    Ok(self
                        .builder
                        .build_int_truncate(int_val, tgt_int_ty, "trunc")
                        .unwrap()
                        .into())
                } else {
                    Ok(val) // same width
                }
            }
            // int → float
            (s, _t) if s.is_integer() && target_ty.is_float() => {
                let int_val = val.into_int_value();
                let float_ty = target_llvm.into_float_type();
                if s.is_signed() {
                    Ok(self
                        .builder
                        .build_signed_int_to_float(int_val, float_ty, "sitofp")
                        .unwrap()
                        .into())
                } else {
                    Ok(self
                        .builder
                        .build_unsigned_int_to_float(int_val, float_ty, "uitofp")
                        .unwrap()
                        .into())
                }
            }
            // float → int
            (_s, t) if source_ty.is_float() && t.is_integer() => {
                let float_val = val.into_float_value();
                let int_ty = target_llvm.into_int_type();
                if t.is_signed() {
                    Ok(self
                        .builder
                        .build_float_to_signed_int(float_val, int_ty, "fptosi")
                        .unwrap()
                        .into())
                } else {
                    Ok(self
                        .builder
                        .build_float_to_unsigned_int(float_val, int_ty, "fptoui")
                        .unwrap()
                        .into())
                }
            }
            // float → float
            (_s, _t) if source_ty.is_float() && target_ty.is_float() => {
                let float_val = val.into_float_value();
                let float_ty = target_llvm.into_float_type();
                if matches!(target_ty, NyType::F64) {
                    Ok(self
                        .builder
                        .build_float_ext(float_val, float_ty, "fpext")
                        .unwrap()
                        .into())
                } else {
                    Ok(self
                        .builder
                        .build_float_trunc(float_val, float_ty, "fptrunc")
                        .unwrap()
                        .into())
                }
            }
            // bool → int
            (NyType::Bool, t) if t.is_integer() => {
                let bool_val = val.into_int_value();
                let int_ty = target_llvm.into_int_type();
                Ok(self
                    .builder
                    .build_int_z_extend(bool_val, int_ty, "boolext")
                    .unwrap()
                    .into())
            }
            _ => Err(vec![CompileError::type_error(
                format!("unsupported cast from {} to {}", source_ty, target_ty),
                Span::empty(0),
            )]),
        }
    }

    fn int_bit_width(&self, ty: &NyType) -> u32 {
        match ty {
            NyType::I8 | NyType::U8 => 8,
            NyType::I16 | NyType::U16 => 16,
            NyType::I32 | NyType::U32 => 32,
            NyType::I64 | NyType::U64 => 64,
            NyType::I128 | NyType::U128 => 128,
            NyType::Bool => 1,
            _ => 32,
        }
    }

    // ------------------------------------------------------------------
    // Compile an expression and return a pointer to its storage.
    // Used for &expr, array indexing (need alloca ptr), field access, etc.
    // ------------------------------------------------------------------

    fn compile_expr_as_ptr(
        &mut self,
        expr: &Expr,
        function: &FunctionValue<'ctx>,
    ) -> Result<PointerValue<'ctx>, Vec<CompileError>> {
        match expr {
            Expr::Ident { name, .. } => {
                if let Some((ptr, _)) = self.variables.get(name) {
                    Ok(*ptr)
                } else {
                    Err(vec![CompileError::name_error(
                        format!("undeclared variable '{}'", name),
                        expr.span(),
                    )])
                }
            }
            Expr::Index { object, index, .. } => {
                // Return the GEP pointer to the element
                let obj_ty = self.infer_expr_type(object);
                let idx_val = self
                    .compile_expr(index, function)?
                    .unwrap()
                    .into_int_value();

                match &obj_ty {
                    NyType::Array { elem, size } => {
                        let elem_llvm = ny_to_llvm(self.context, elem);
                        let arr_llvm = elem_llvm.array_type(*size as u32);
                        let arr_ptr = self.compile_expr_as_ptr(object, function)?;

                        let arr_len = self.context.i64_type().const_int(*size as u64, false);
                        let idx_i64 = self
                            .builder
                            .build_int_z_extend_or_bit_cast(
                                idx_val,
                                self.context.i64_type(),
                                "idx_ext",
                            )
                            .unwrap();
                        self.build_bounds_check(idx_i64, arr_len, function);

                        let zero = self.context.i64_type().const_zero();
                        let gep = unsafe {
                            self.builder
                                .build_in_bounds_gep(arr_llvm, arr_ptr, &[zero, idx_i64], "idx_ptr")
                                .unwrap()
                        };
                        Ok(gep)
                    }
                    _ => Err(vec![CompileError::type_error(
                        "cannot index non-array type".to_string(),
                        object.span(),
                    )]),
                }
            }
            Expr::FieldAccess { object, field, .. } => {
                let obj_ty = self.infer_expr_type(object);

                let (struct_name, struct_fields, is_pointer) = match &obj_ty {
                    NyType::Struct { name, fields } => (name.clone(), fields.clone(), false),
                    NyType::Pointer(inner) => match inner.as_ref() {
                        NyType::Struct { name, fields } => (name.clone(), fields.clone(), true),
                        _ => {
                            return Err(vec![CompileError::type_error(
                                "field access on non-struct pointer".to_string(),
                                object.span(),
                            )]);
                        }
                    },
                    _ => {
                        return Err(vec![CompileError::type_error(
                            format!("field access on non-struct type '{}'", obj_ty),
                            object.span(),
                        )]);
                    }
                };

                let field_idx = struct_fields
                    .iter()
                    .position(|(n, _)| n == field)
                    .unwrap_or(0) as u32;
                let struct_llvm_ty =
                    self.get_or_create_llvm_struct_type(&struct_name, &struct_fields);

                let struct_ptr = if is_pointer {
                    let ptr_val = self.compile_expr(object, function)?.unwrap();
                    ptr_val.into_pointer_value()
                } else {
                    self.compile_expr_as_ptr(object, function)?
                };

                let field_ptr = self
                    .builder
                    .build_struct_gep(
                        struct_llvm_ty,
                        struct_ptr,
                        field_idx,
                        &format!("{}_field_ptr", field),
                    )
                    .unwrap();
                Ok(field_ptr)
            }
            Expr::Deref { operand, .. } => {
                // *ptr — the pointer itself is the address we want
                let ptr_val = self.compile_expr(operand, function)?.unwrap();
                Ok(ptr_val.into_pointer_value())
            }
            _ => {
                // For arbitrary expressions, evaluate and store in a temporary alloca
                let val = self.compile_expr(expr, function)?.unwrap();
                let alloca = self.builder.build_alloca(val.get_type(), "tmp").unwrap();
                self.builder.build_store(alloca, val).unwrap();
                Ok(alloca)
            }
        }
    }

    // ------------------------------------------------------------------
    // Compile statements
    // ------------------------------------------------------------------

    fn compile_stmt(
        &mut self,
        stmt: &Stmt,
        function: &FunctionValue<'ctx>,
    ) -> Result<(), Vec<CompileError>> {
        match stmt {
            Stmt::VarDecl { name, ty, init, .. } => {
                let val = self.compile_expr(init, function)?.unwrap();
                let ny_ty = if let Some(t) = ty {
                    self.resolve_type_annotation(t)
                } else {
                    self.infer_expr_type(init)
                };
                let llvm_ty = ny_to_llvm(self.context, &ny_ty);
                let alloca = self.builder.build_alloca(llvm_ty, name).unwrap();
                self.builder.build_store(alloca, val).unwrap();
                self.variables.insert(name.clone(), (alloca, ny_ty));

                // If this is a capturing lambda, transfer closure info to var name
                if let Expr::Lambda { .. } = init {
                    // Find which lambda was just compiled (most recent one)
                    let recent_lambda: Option<String> = self
                        .closure_captures
                        .keys()
                        .filter(|k| k.starts_with("__lambda_"))
                        .max()
                        .cloned();
                    if let Some(lambda_key) = recent_lambda {
                        if let Some(info) = self.closure_captures.remove(&lambda_key) {
                            self.closure_captures.insert(name.clone(), info);
                        }
                    }
                }

                Ok(())
            }
            Stmt::ConstDecl {
                name, ty, value, ..
            } => {
                let val = self.compile_expr(value, function)?.unwrap();
                let ny_ty = if let Some(t) = ty {
                    self.resolve_type_annotation(t)
                } else {
                    self.infer_expr_type(value)
                };
                let llvm_ty = ny_to_llvm(self.context, &ny_ty);
                let alloca = self.builder.build_alloca(llvm_ty, name).unwrap();
                self.builder.build_store(alloca, val).unwrap();
                self.variables.insert(name.clone(), (alloca, ny_ty));
                Ok(())
            }
            Stmt::Assign { target, value, .. } => {
                let val = self.compile_expr(value, function)?.unwrap();
                self.compile_assign_target(target, val, function)?;
                Ok(())
            }
            Stmt::ExprStmt { expr, .. } => {
                self.compile_expr(expr, function)?;
                Ok(())
            }
            Stmt::Return { value, .. } => {
                // Evaluate the return value first
                let ret_val = if let Some(val_expr) = value {
                    self.compile_expr(val_expr, function)?
                } else {
                    None
                };

                // Emit all deferred expressions in LIFO order before returning
                let defers: Vec<(Expr, FunctionValue<'ctx>)> =
                    self.defer_stack.iter().rev().cloned().collect();
                for (defer_body, defer_fn) in &defers {
                    self.compile_expr(defer_body, defer_fn)?;
                }

                if let Some(v) = ret_val {
                    self.builder.build_return(Some(&v)).unwrap();
                } else {
                    self.builder.build_return(None).unwrap();
                }
                Ok(())
            }

            // ---- While loop (now with loop_stack for break/continue) ----
            Stmt::While {
                condition, body, ..
            } => {
                let cond_bb = self.context.append_basic_block(*function, "while_cond");
                let body_bb = self.context.append_basic_block(*function, "while_body");
                let exit_bb = self.context.append_basic_block(*function, "while_exit");

                self.builder.build_unconditional_branch(cond_bb).unwrap();

                // Push loop frame (break -> exit_bb, continue -> cond_bb)
                self.loop_stack.push(LoopFrame {
                    break_bb: exit_bb,
                    continue_bb: cond_bb,
                });

                // Condition
                self.builder.position_at_end(cond_bb);
                let cond_val = self.compile_expr(condition, function)?.unwrap();
                self.builder
                    .build_conditional_branch(cond_val.into_int_value(), body_bb, exit_bb)
                    .unwrap();

                // Body
                self.builder.position_at_end(body_bb);
                self.compile_expr(body, function)?;
                if self
                    .builder
                    .get_insert_block()
                    .unwrap()
                    .get_terminator()
                    .is_none()
                {
                    self.builder.build_unconditional_branch(cond_bb).unwrap();
                }

                self.loop_stack.pop();

                // Exit
                self.builder.position_at_end(exit_bb);
                Ok(())
            }

            // ---- For-range loop ----
            Stmt::ForRange {
                var,
                start,
                end,
                inclusive,
                body,
                ..
            } => {
                // Evaluate start and end
                let start_val = self
                    .compile_expr(start, function)?
                    .unwrap()
                    .into_int_value();
                let end_val = self.compile_expr(end, function)?.unwrap().into_int_value();

                // Allocate the loop variable
                let i32_ty = self.context.i32_type();
                let loop_var = self.builder.build_alloca(i32_ty, var).unwrap();
                self.builder.build_store(loop_var, start_val).unwrap();
                self.variables.insert(var.clone(), (loop_var, NyType::I32));

                let cond_bb = self.context.append_basic_block(*function, "for_cond");
                let body_bb = self.context.append_basic_block(*function, "for_body");
                let inc_bb = self.context.append_basic_block(*function, "for_inc");
                let exit_bb = self.context.append_basic_block(*function, "for_exit");

                self.builder.build_unconditional_branch(cond_bb).unwrap();

                // Push loop frame (break -> exit_bb, continue -> inc_bb)
                self.loop_stack.push(LoopFrame {
                    break_bb: exit_bb,
                    continue_bb: inc_bb,
                });

                // Condition block
                self.builder.position_at_end(cond_bb);
                let current_val = self
                    .builder
                    .build_load(i32_ty, loop_var, "loop_var")
                    .unwrap()
                    .into_int_value();
                let cmp = if *inclusive {
                    // <=
                    self.builder
                        .build_int_compare(IntPredicate::SLE, current_val, end_val, "for_cond")
                        .unwrap()
                } else {
                    // <
                    self.builder
                        .build_int_compare(IntPredicate::SLT, current_val, end_val, "for_cond")
                        .unwrap()
                };
                self.builder
                    .build_conditional_branch(cmp, body_bb, exit_bb)
                    .unwrap();

                // Body block
                self.builder.position_at_end(body_bb);
                self.compile_expr(body, function)?;
                if self
                    .builder
                    .get_insert_block()
                    .unwrap()
                    .get_terminator()
                    .is_none()
                {
                    self.builder.build_unconditional_branch(inc_bb).unwrap();
                }

                // Increment block
                self.builder.position_at_end(inc_bb);
                let inc_val = self
                    .builder
                    .build_load(i32_ty, loop_var, "loop_var")
                    .unwrap()
                    .into_int_value();
                let next_val = self
                    .builder
                    .build_int_add(inc_val, i32_ty.const_int(1, false), "inc")
                    .unwrap();
                self.builder.build_store(loop_var, next_val).unwrap();
                self.builder.build_unconditional_branch(cond_bb).unwrap();

                self.loop_stack.pop();

                // Exit block
                self.builder.position_at_end(exit_bb);
                Ok(())
            }

            // ---- ForIn (desugar: for var in collection → index loop) ----
            Stmt::ForIn {
                var,
                collection,
                body,
                ..
            } => {
                let coll_ty = self.infer_expr_type(collection);
                let (elem_ty, coll_len) = match &coll_ty {
                    NyType::Array { elem, size } => {
                        (*elem.clone(), self.context.i32_type().const_int(*size as u64, false))
                    }
                    NyType::Slice(_) | NyType::Vec(_) => {
                        // Get len dynamically
                        let coll_val = self.compile_expr(collection, function)?.unwrap();
                        let sv = coll_val.into_struct_value();
                        let len = self
                            .builder
                            .build_extract_value(sv, 1, "forin_len")
                            .unwrap()
                            .into_int_value();
                        let len_i32 = self
                            .builder
                            .build_int_truncate(len, self.context.i32_type(), "len_i32")
                            .unwrap();
                        let elem = match &coll_ty {
                            NyType::Slice(e) => *e.clone(),
                            NyType::Vec(e) => *e.clone(),
                            _ => NyType::I32,
                        };
                        (elem, len_i32)
                    }
                    _ => {
                        return Err(vec![CompileError::type_error(
                            format!("cannot iterate over '{}'", coll_ty),
                            collection.span(),
                        )]);
                    }
                };

                let elem_llvm = ny_to_llvm(self.context, &elem_ty);
                let i32_ty = self.context.i32_type();

                // Allocate loop index
                let idx_alloca = self.builder.build_alloca(i32_ty, "__forin_idx").unwrap();
                self.builder
                    .build_store(idx_alloca, i32_ty.const_zero())
                    .unwrap();

                // Allocate element variable
                let var_alloca = self.builder.build_alloca(elem_llvm, var).unwrap();
                self.variables
                    .insert(var.clone(), (var_alloca, elem_ty.clone()));

                let cond_bb = self.context.append_basic_block(*function, "forin_cond");
                let body_bb = self.context.append_basic_block(*function, "forin_body");
                let inc_bb = self.context.append_basic_block(*function, "forin_inc");
                let exit_bb = self.context.append_basic_block(*function, "forin_exit");

                self.builder.build_unconditional_branch(cond_bb).unwrap();

                self.loop_stack.push(LoopFrame {
                    break_bb: exit_bb,
                    continue_bb: inc_bb,
                });

                // Condition: idx < len
                self.builder.position_at_end(cond_bb);
                let idx = self
                    .builder
                    .build_load(i32_ty, idx_alloca, "idx")
                    .unwrap()
                    .into_int_value();
                let cmp = self
                    .builder
                    .build_int_compare(IntPredicate::SLT, idx, coll_len, "forin_cmp")
                    .unwrap();
                self.builder
                    .build_conditional_branch(cmp, body_bb, exit_bb)
                    .unwrap();

                // Body: load element, execute body
                self.builder.position_at_end(body_bb);
                // Get element from collection[idx]
                match &coll_ty {
                    NyType::Array { elem, size } => {
                        let arr_ptr = self.compile_expr_as_ptr(collection, function)?;
                        let elem_llvm_inner = ny_to_llvm(self.context, elem);
                        let arr_llvm_ty = elem_llvm_inner.array_type(*size as u32);
                        let cur_idx = self
                            .builder
                            .build_load(i32_ty, idx_alloca, "cur_idx")
                            .unwrap()
                            .into_int_value();
                        let idx_i64 = self
                            .builder
                            .build_int_z_extend_or_bit_cast(
                                cur_idx,
                                self.context.i64_type(),
                                "idx64",
                            )
                            .unwrap();
                        let zero = self.context.i64_type().const_zero();
                        let gep = unsafe {
                            self.builder
                                .build_in_bounds_gep(
                                    arr_llvm_ty,
                                    arr_ptr,
                                    &[zero, idx_i64],
                                    "forin_gep",
                                )
                                .unwrap()
                        };
                        let elem_val = self
                            .builder
                            .build_load(elem_llvm, gep, "elem")
                            .unwrap();
                        self.builder.build_store(var_alloca, elem_val).unwrap();
                    }
                    NyType::Slice(_) | NyType::Vec(_) => {
                        // For slices/vecs, get the data pointer
                        let coll_val = self.compile_expr(collection, function)?.unwrap();
                        let sv = coll_val.into_struct_value();
                        let data_ptr = self
                            .builder
                            .build_extract_value(sv, 0, "data_ptr")
                            .unwrap()
                            .into_pointer_value();
                        let cur_idx = self
                            .builder
                            .build_load(i32_ty, idx_alloca, "cur_idx")
                            .unwrap()
                            .into_int_value();
                        let idx_i64 = self
                            .builder
                            .build_int_z_extend_or_bit_cast(
                                cur_idx,
                                self.context.i64_type(),
                                "idx64",
                            )
                            .unwrap();
                        let gep = unsafe {
                            self.builder
                                .build_in_bounds_gep(elem_llvm, data_ptr, &[idx_i64], "forin_gep")
                                .unwrap()
                        };
                        let elem_val = self
                            .builder
                            .build_load(elem_llvm, gep, "elem")
                            .unwrap();
                        self.builder.build_store(var_alloca, elem_val).unwrap();
                    }
                    _ => {}
                }

                self.compile_expr(body, function)?;
                if self
                    .builder
                    .get_insert_block()
                    .unwrap()
                    .get_terminator()
                    .is_none()
                {
                    self.builder.build_unconditional_branch(inc_bb).unwrap();
                }

                // Increment
                self.builder.position_at_end(inc_bb);
                let cur = self
                    .builder
                    .build_load(i32_ty, idx_alloca, "cur")
                    .unwrap()
                    .into_int_value();
                let next = self
                    .builder
                    .build_int_add(cur, i32_ty.const_int(1, false), "next")
                    .unwrap();
                self.builder.build_store(idx_alloca, next).unwrap();
                self.builder.build_unconditional_branch(cond_bb).unwrap();

                self.loop_stack.pop();
                self.builder.position_at_end(exit_bb);
                Ok(())
            }

            // ---- Break ----
            Stmt::Break { .. } => {
                if let Some(frame) = self.loop_stack.last() {
                    let break_bb = frame.break_bb;
                    self.builder.build_unconditional_branch(break_bb).unwrap();
                }
                Ok(())
            }

            // ---- Continue ----
            Stmt::Continue { .. } => {
                if let Some(frame) = self.loop_stack.last() {
                    let continue_bb = frame.continue_bb;
                    self.builder
                        .build_unconditional_branch(continue_bb)
                        .unwrap();
                }
                Ok(())
            }

            Stmt::WhileLet {
                pattern,
                expr: match_expr,
                body: while_body,
                span: while_span,
            } => {
                // Desugar: loop { match expr { Pattern => body, _ => break } }
                let loop_body_bb = self.context.append_basic_block(*function, "whilelet_body");
                let exit_bb = self.context.append_basic_block(*function, "whilelet_exit");

                self.builder.build_unconditional_branch(loop_body_bb).unwrap();
                self.loop_stack.push(LoopFrame { break_bb: exit_bb, continue_bb: loop_body_bb });

                self.builder.position_at_end(loop_body_bb);

                // Build: match expr { pattern => body, _ => break }
                let break_body = Expr::Block {
                    stmts: vec![Stmt::Break { span: *while_span }],
                    tail_expr: None,
                    span: *while_span,
                };
                let match_ast = Expr::Match {
                    subject: Box::new(match_expr.clone()),
                    arms: vec![
                        MatchArm { pattern: pattern.clone(), body: while_body.clone() },
                        MatchArm { pattern: Pattern::Wildcard(Span::empty(0)), body: break_body },
                    ],
                    span: *while_span,
                };
                self.compile_expr(&match_ast, function)?;

                if self.builder.get_insert_block().unwrap().get_terminator().is_none() {
                    self.builder.build_unconditional_branch(loop_body_bb).unwrap();
                }

                self.loop_stack.pop();
                self.builder.position_at_end(exit_bb);
                Ok(())
            }

            Stmt::IfLet {
                pattern,
                expr: match_expr,
                then_body,
                else_body,
                ..
            } => {
                // Desugar if let to: match expr { pattern => then, _ => else }
                let wildcard_body = if let Some(eb) = else_body {
                    eb.clone()
                } else {
                    Expr::Block {
                        stmts: Vec::new(),
                        tail_expr: None,
                        span: Span::empty(0),
                    }
                };
                let match_expr_ast = Expr::Match {
                    subject: Box::new(match_expr.clone()),
                    arms: vec![
                        MatchArm {
                            pattern: pattern.clone(),
                            body: then_body.clone(),
                        },
                        MatchArm {
                            pattern: Pattern::Wildcard(Span::empty(0)),
                            body: wildcard_body,
                        },
                    ],
                    span: Span::empty(0),
                };
                self.compile_expr(&match_expr_ast, function)?;
                Ok(())
            }

            Stmt::Loop { body, .. } => {
                let body_bb = self.context.append_basic_block(*function, "loop_body");
                let exit_bb = self.context.append_basic_block(*function, "loop_exit");

                self.builder.build_unconditional_branch(body_bb).unwrap();

                self.loop_stack.push(LoopFrame {
                    break_bb: exit_bb,
                    continue_bb: body_bb,
                });

                self.builder.position_at_end(body_bb);
                self.compile_expr(body, function)?;
                if self
                    .builder
                    .get_insert_block()
                    .unwrap()
                    .get_terminator()
                    .is_none()
                {
                    self.builder.build_unconditional_branch(body_bb).unwrap();
                }

                self.loop_stack.pop();
                self.builder.position_at_end(exit_bb);
                Ok(())
            }

            Stmt::Defer { body, .. } => {
                // Push the deferred expression onto the stack; it will be emitted
                // when the function returns or the scope exits.
                self.defer_stack.push((body.clone(), *function));
                Ok(())
            }

            Stmt::TupleDestructure { names, init, .. } => {
                let val = self.compile_expr(init, function)?.unwrap();
                let struct_val = val.into_struct_value();
                let tuple_ty = self.infer_expr_type(init);
                let elem_types = match &tuple_ty {
                    NyType::Tuple(elems) => elems.clone(),
                    _ => vec![NyType::I32; names.len()],
                };

                for (i, name) in names.iter().enumerate() {
                    let elem_val = self
                        .builder
                        .build_extract_value(struct_val, i as u32, name)
                        .unwrap();
                    let elem_ty = elem_types.get(i).cloned().unwrap_or(NyType::I32);
                    let llvm_ty = ny_to_llvm(self.context, &elem_ty);
                    let alloca = self.builder.build_alloca(llvm_ty, name).unwrap();
                    self.builder.build_store(alloca, elem_val).unwrap();
                    self.variables.insert(name.clone(), (alloca, elem_ty));
                }
                Ok(())
            }
        }
    }

    // ------------------------------------------------------------------
    // AssignTarget compilation
    // ------------------------------------------------------------------

    fn compile_assign_target(
        &mut self,
        target: &AssignTarget,
        val: BasicValueEnum<'ctx>,
        function: &FunctionValue<'ctx>,
    ) -> Result<(), Vec<CompileError>> {
        match target {
            AssignTarget::Var(name) => {
                if let Some((ptr, _)) = self.variables.get(name) {
                    self.builder.build_store(*ptr, val).unwrap();
                }
                Ok(())
            }
            AssignTarget::Index(object_expr, index_expr) => {
                let obj_ty = self.infer_expr_type(object_expr);
                let idx_val = self
                    .compile_expr(index_expr, function)?
                    .unwrap()
                    .into_int_value();

                match &obj_ty {
                    NyType::Array { elem, size } => {
                        let elem_llvm = ny_to_llvm(self.context, elem);
                        let arr_llvm = elem_llvm.array_type(*size as u32);
                        let arr_ptr = self.compile_expr_as_ptr(object_expr, function)?;

                        let arr_len = self.context.i64_type().const_int(*size as u64, false);
                        let idx_i64 = self
                            .builder
                            .build_int_z_extend_or_bit_cast(
                                idx_val,
                                self.context.i64_type(),
                                "idx_ext",
                            )
                            .unwrap();
                        self.build_bounds_check(idx_i64, arr_len, function);

                        let zero = self.context.i64_type().const_zero();
                        let gep = unsafe {
                            self.builder
                                .build_in_bounds_gep(
                                    arr_llvm,
                                    arr_ptr,
                                    &[zero, idx_i64],
                                    "idx_store_ptr",
                                )
                                .unwrap()
                        };
                        self.builder.build_store(gep, val).unwrap();
                    }
                    _ => {
                        return Err(vec![CompileError::type_error(
                            "cannot index non-array type for assignment".to_string(),
                            object_expr.span(),
                        )]);
                    }
                }
                Ok(())
            }
            AssignTarget::Field(object_expr, field_name) => {
                let obj_ty = self.infer_expr_type(object_expr);

                let (struct_name, struct_fields, is_pointer) = match &obj_ty {
                    NyType::Struct { name, fields } => (name.clone(), fields.clone(), false),
                    NyType::Pointer(inner) => match inner.as_ref() {
                        NyType::Struct { name, fields } => (name.clone(), fields.clone(), true),
                        _ => {
                            return Err(vec![CompileError::type_error(
                                "field assign on non-struct pointer".to_string(),
                                object_expr.span(),
                            )]);
                        }
                    },
                    _ => {
                        return Err(vec![CompileError::type_error(
                            format!("field assign on non-struct type '{}'", obj_ty),
                            object_expr.span(),
                        )]);
                    }
                };

                let field_idx = struct_fields
                    .iter()
                    .position(|(n, _)| n == field_name)
                    .unwrap_or(0) as u32;
                let struct_llvm_ty =
                    self.get_or_create_llvm_struct_type(&struct_name, &struct_fields);

                let struct_ptr = if is_pointer {
                    let ptr_val = self.compile_expr(object_expr, function)?.unwrap();
                    ptr_val.into_pointer_value()
                } else {
                    self.compile_expr_as_ptr(object_expr, function)?
                };

                let field_ptr = self
                    .builder
                    .build_struct_gep(
                        struct_llvm_ty,
                        struct_ptr,
                        field_idx,
                        &format!("{}_assign_ptr", field_name),
                    )
                    .unwrap();
                self.builder.build_store(field_ptr, val).unwrap();
                Ok(())
            }
            AssignTarget::Deref(operand_expr) => {
                let ptr_val = self.compile_expr(operand_expr, function)?.unwrap();
                let ptr = ptr_val.into_pointer_value();
                self.builder.build_store(ptr, val).unwrap();
                Ok(())
            }
        }
    }

    // ------------------------------------------------------------------
    // Bounds checking for array index operations
    // ------------------------------------------------------------------

    fn build_bounds_check(
        &self,
        index: inkwell::values::IntValue<'ctx>,
        length: inkwell::values::IntValue<'ctx>,
        function: &FunctionValue<'ctx>,
    ) {
        let in_bounds = self
            .builder
            .build_int_compare(IntPredicate::ULT, index, length, "bounds_check")
            .unwrap();

        let ok_bb = self.context.append_basic_block(*function, "bounds_ok");
        let fail_bb = self.context.append_basic_block(*function, "bounds_fail");

        self.builder
            .build_conditional_branch(in_bounds, ok_bb, fail_bb)
            .unwrap();

        // Fail block: print error message with index and length, then exit(1)
        self.builder.position_at_end(fail_bb);
        let fprintf_fn = self.get_or_declare_fprintf();
        let stderr_global = self.get_or_declare_stderr();
        let stderr_val = self
            .builder
            .build_load(
                self.context.ptr_type(AddressSpace::default()),
                stderr_global.as_pointer_value(),
                "stderr",
            )
            .unwrap();
        let fmt = self
            .builder
            .build_global_string_ptr(
                "panic: index out of bounds: index %ld, length %ld\n",
                "bounds_fmt",
            )
            .unwrap();
        // Extend index and length to i64 for printing
        let idx_i64 = self
            .builder
            .build_int_z_extend_or_bit_cast(index, self.context.i64_type(), "idx_print")
            .unwrap();
        let len_i64 = self
            .builder
            .build_int_z_extend_or_bit_cast(length, self.context.i64_type(), "len_print")
            .unwrap();
        self.builder
            .build_call(
                fprintf_fn,
                &[
                    stderr_val.into(),
                    fmt.as_pointer_value().into(),
                    idx_i64.into(),
                    len_i64.into(),
                ],
                "",
            )
            .unwrap();
        let exit_fn = self.get_or_declare_exit();
        self.builder
            .build_call(exit_fn, &[self.context.i32_type().const_int(1, false).into()], "")
            .unwrap();
        self.builder.build_unreachable().unwrap();

        // Continue from ok block
        self.builder.position_at_end(ok_bb);
    }

    fn get_or_declare_fprintf(&self) -> FunctionValue<'ctx> {
        if let Some(f) = self.module.get_function("fprintf") {
            return f;
        }
        let ptr_ty = self.context.ptr_type(AddressSpace::default());
        let i32_ty = self.context.i32_type();
        let fprintf_ty = i32_ty.fn_type(&[ptr_ty.into(), ptr_ty.into()], true);
        self.module.add_function("fprintf", fprintf_ty, None)
    }

    fn get_or_declare_stderr(&self) -> inkwell::values::GlobalValue<'ctx> {
        if let Some(g) = self.module.get_global("stderr") {
            return g;
        }
        let ptr_ty = self.context.ptr_type(AddressSpace::default());
        self.module.add_global(ptr_ty, None, "stderr")
    }

    // ------------------------------------------------------------------
    // String literal: build a global constant and return {ptr, len} struct
    // ------------------------------------------------------------------

    fn build_str_literal(&self, s: &str) -> BasicValueEnum<'ctx> {
        let str_ty = str_type(self.context);
        let global = self.builder.build_global_string_ptr(s, "str_lit").unwrap();
        let ptr_val = global.as_pointer_value();
        let len_val = self.context.i64_type().const_int(s.len() as u64, false);

        // Build the {ptr, len} struct
        let str_val = str_ty.const_zero();
        let str_val = self
            .builder
            .build_insert_value(str_val, ptr_val, 0, "str_ptr")
            .unwrap();
        let str_val = self
            .builder
            .build_insert_value(str_val, len_val, 1, "str_len")
            .unwrap();
        str_val.into_struct_value().into()
    }

    // ------------------------------------------------------------------
    // print/println builtins
    // ------------------------------------------------------------------

    fn compile_print_call(
        &mut self,
        callee: &str,
        args: &[Expr],
        function: &FunctionValue<'ctx>,
    ) -> Result<(), Vec<CompileError>> {
        let is_println = callee == "println";

        for arg in args {
            let arg_ty = self.infer_expr_type(arg);
            let val = self.compile_expr(arg, function)?;

            match &arg_ty {
                NyType::I32 => {
                    let printf_fn = self.get_or_declare_printf();
                    let fmt = self
                        .builder
                        .build_global_string_ptr("%d", "fmt_i32")
                        .unwrap();
                    self.builder
                        .build_call(
                            printf_fn,
                            &[fmt.as_pointer_value().into(), val.unwrap().into()],
                            "",
                        )
                        .unwrap();
                }
                NyType::I64 => {
                    let printf_fn = self.get_or_declare_printf();
                    let fmt = self
                        .builder
                        .build_global_string_ptr("%ld", "fmt_i64")
                        .unwrap();
                    self.builder
                        .build_call(
                            printf_fn,
                            &[fmt.as_pointer_value().into(), val.unwrap().into()],
                            "",
                        )
                        .unwrap();
                }
                t if t.is_integer() => {
                    // All other integer types: print as %ld after sign/zero extending to i64
                    let printf_fn = self.get_or_declare_printf();
                    let fmt = self
                        .builder
                        .build_global_string_ptr("%ld", "fmt_int")
                        .unwrap();
                    let int_val = val.unwrap().into_int_value();
                    let i64_val = if t.is_signed() {
                        self.builder
                            .build_int_s_extend(int_val, self.context.i64_type(), "sext")
                            .unwrap()
                    } else {
                        self.builder
                            .build_int_z_extend(int_val, self.context.i64_type(), "zext")
                            .unwrap()
                    };
                    self.builder
                        .build_call(
                            printf_fn,
                            &[fmt.as_pointer_value().into(), i64_val.into()],
                            "",
                        )
                        .unwrap();
                }
                NyType::F64 | NyType::F32 => {
                    let printf_fn = self.get_or_declare_printf();
                    let fmt = self
                        .builder
                        .build_global_string_ptr("%f", "fmt_float")
                        .unwrap();
                    // Ensure f32 is promoted to f64 for printf (varargs convention)
                    let float_val = if arg_ty == NyType::F32 {
                        let f32_val = val.unwrap().into_float_value();
                        self.builder
                            .build_float_ext(f32_val, self.context.f64_type(), "fpext")
                            .unwrap()
                            .into()
                    } else {
                        val.unwrap()
                    };
                    self.builder
                        .build_call(
                            printf_fn,
                            &[fmt.as_pointer_value().into(), float_val.into()],
                            "",
                        )
                        .unwrap();
                }
                NyType::Bool => {
                    // Print "true" or "false"
                    let bool_val = val.unwrap().into_int_value();
                    let true_str = self
                        .builder
                        .build_global_string_ptr("true", "true_str")
                        .unwrap();
                    let false_str = self
                        .builder
                        .build_global_string_ptr("false", "false_str")
                        .unwrap();
                    let selected = self
                        .builder
                        .build_select(
                            bool_val,
                            true_str.as_pointer_value(),
                            false_str.as_pointer_value(),
                            "bool_str",
                        )
                        .unwrap();
                    let printf_fn = self.get_or_declare_printf();
                    let fmt = self
                        .builder
                        .build_global_string_ptr("%s", "fmt_bool")
                        .unwrap();
                    self.builder
                        .build_call(
                            printf_fn,
                            &[fmt.as_pointer_value().into(), selected.into()],
                            "",
                        )
                        .unwrap();
                }
                NyType::Str => {
                    // str is {ptr, len} — use write(1, ptr, len)
                    let write_fn = self.get_or_declare_write();
                    let str_val = val.unwrap().into_struct_value();
                    let ptr = self
                        .builder
                        .build_extract_value(str_val, 0, "str_ptr")
                        .unwrap();
                    let len = self
                        .builder
                        .build_extract_value(str_val, 1, "str_len")
                        .unwrap();
                    let fd = self.context.i32_type().const_int(1, false); // stdout
                    self.builder
                        .build_call(write_fn, &[fd.into(), ptr.into(), len.into()], "")
                        .unwrap();
                }
                NyType::Struct { name, fields } => {
                    // Print struct as StructName { field1: val1, field2: val2, ... }
                    let printf_fn = self.get_or_declare_printf();
                    let struct_val = val.unwrap().into_struct_value();

                    // Print "StructName { "
                    let header = self
                        .builder
                        .build_global_string_ptr(&format!("{} {{ ", name), "struct_header")
                        .unwrap();
                    let fmt_s = self.builder.build_global_string_ptr("%s", "fmt_s").unwrap();
                    self.builder
                        .build_call(
                            printf_fn,
                            &[
                                fmt_s.as_pointer_value().into(),
                                header.as_pointer_value().into(),
                            ],
                            "",
                        )
                        .unwrap();

                    for (i, (fname, ftype)) in fields.iter().enumerate() {
                        let field_val = self
                            .builder
                            .build_extract_value(struct_val, i as u32, fname)
                            .unwrap();

                        // Print "field_name: "
                        let prefix = if i > 0 {
                            format!(", {}: ", fname)
                        } else {
                            format!("{}: ", fname)
                        };
                        let prefix_str = self
                            .builder
                            .build_global_string_ptr(&prefix, "field_prefix")
                            .unwrap();
                        self.builder
                            .build_call(
                                printf_fn,
                                &[
                                    fmt_s.as_pointer_value().into(),
                                    prefix_str.as_pointer_value().into(),
                                ],
                                "",
                            )
                            .unwrap();

                        // Print the field value based on its type
                        match ftype {
                            NyType::I32 => {
                                let fmt_d =
                                    self.builder.build_global_string_ptr("%d", "fmt_d").unwrap();
                                self.builder
                                    .build_call(
                                        printf_fn,
                                        &[fmt_d.as_pointer_value().into(), field_val.into()],
                                        "",
                                    )
                                    .unwrap();
                            }
                            NyType::I64 => {
                                let fmt_ld = self
                                    .builder
                                    .build_global_string_ptr("%ld", "fmt_ld")
                                    .unwrap();
                                self.builder
                                    .build_call(
                                        printf_fn,
                                        &[fmt_ld.as_pointer_value().into(), field_val.into()],
                                        "",
                                    )
                                    .unwrap();
                            }
                            NyType::F64 | NyType::F32 => {
                                let fmt_f =
                                    self.builder.build_global_string_ptr("%f", "fmt_f").unwrap();
                                self.builder
                                    .build_call(
                                        printf_fn,
                                        &[fmt_f.as_pointer_value().into(), field_val.into()],
                                        "",
                                    )
                                    .unwrap();
                            }
                            NyType::Bool => {
                                let bool_v = field_val.into_int_value();
                                let ts = self.builder.build_global_string_ptr("true", "t").unwrap();
                                let fs =
                                    self.builder.build_global_string_ptr("false", "f").unwrap();
                                let sel = self
                                    .builder
                                    .build_select(
                                        bool_v,
                                        ts.as_pointer_value(),
                                        fs.as_pointer_value(),
                                        "bs",
                                    )
                                    .unwrap();
                                let fmt_bs = self
                                    .builder
                                    .build_global_string_ptr("%s", "fmt_bs")
                                    .unwrap();
                                self.builder
                                    .build_call(
                                        printf_fn,
                                        &[fmt_bs.as_pointer_value().into(), sel.into()],
                                        "",
                                    )
                                    .unwrap();
                            }
                            _ => {
                                // Fallback: print as integer
                                let fmt_d = self
                                    .builder
                                    .build_global_string_ptr("%d", "fmt_d_fb")
                                    .unwrap();
                                self.builder
                                    .build_call(
                                        printf_fn,
                                        &[fmt_d.as_pointer_value().into(), field_val.into()],
                                        "",
                                    )
                                    .unwrap();
                            }
                        }
                    }

                    // Print " }"
                    let footer = self
                        .builder
                        .build_global_string_ptr(" }", "struct_footer")
                        .unwrap();
                    self.builder
                        .build_call(
                            printf_fn,
                            &[
                                fmt_s.as_pointer_value().into(),
                                footer.as_pointer_value().into(),
                            ],
                            "",
                        )
                        .unwrap();
                }
                NyType::Enum { name, variants } => {
                    // Print enum variant name using a switch on the discriminant
                    let printf_fn = self.get_or_declare_printf();
                    let fmt_s = self
                        .builder
                        .build_global_string_ptr("%s", "fmt_enum")
                        .unwrap();
                    let disc_val = val.unwrap().into_int_value();

                    // Extract variant names for printing
                    let variant_names: Vec<String> =
                        variants.iter().map(|(name, _)| name.clone()).collect();

                    // Capture the current block before creating new ones
                    let origin_bb = self.builder.get_insert_block().unwrap();

                    let merge_bb = self
                        .context
                        .append_basic_block(*function, "enum_print_merge");
                    let default_bb = self
                        .context
                        .append_basic_block(*function, "enum_print_default");

                    // Create basic blocks for each variant
                    let variant_bbs: Vec<BasicBlock> = variant_names
                        .iter()
                        .enumerate()
                        .map(|(i, _)| {
                            self.context
                                .append_basic_block(*function, &format!("enum_print_{}", i))
                        })
                        .collect();

                    // Build default block: print "EnumName::?" for unknown discriminant
                    self.builder.position_at_end(default_bb);
                    let unknown_str = self
                        .builder
                        .build_global_string_ptr(&format!("{}::?", name), "enum_unknown")
                        .unwrap();
                    self.builder
                        .build_call(
                            printf_fn,
                            &[
                                fmt_s.as_pointer_value().into(),
                                unknown_str.as_pointer_value().into(),
                            ],
                            "",
                        )
                        .unwrap();
                    self.builder.build_unconditional_branch(merge_bb).unwrap();

                    // Build each variant print block
                    for (i, variant_name) in variant_names.iter().enumerate() {
                        self.builder.position_at_end(variant_bbs[i]);
                        let var_str = self
                            .builder
                            .build_global_string_ptr(
                                &format!("{}::{}", name, variant_name),
                                &format!("enum_var_{}", i),
                            )
                            .unwrap();
                        self.builder
                            .build_call(
                                printf_fn,
                                &[
                                    fmt_s.as_pointer_value().into(),
                                    var_str.as_pointer_value().into(),
                                ],
                                "",
                            )
                            .unwrap();
                        self.builder.build_unconditional_branch(merge_bb).unwrap();
                    }

                    // Go back to the origin block and emit the switch
                    self.builder.position_at_end(origin_bb);
                    let cases: Vec<(inkwell::values::IntValue, BasicBlock)> = variant_names
                        .iter()
                        .enumerate()
                        .map(|(i, _)| {
                            (
                                self.context.i32_type().const_int(i as u64, false),
                                variant_bbs[i],
                            )
                        })
                        .collect();
                    self.builder
                        .build_switch(disc_val, default_bb, &cases)
                        .unwrap();

                    self.builder.position_at_end(merge_bb);
                }
                NyType::Tuple(ref elem_types) => {
                    // Print tuple as (elem1, elem2, ...)
                    let printf_fn = self.get_or_declare_printf();
                    let tuple_val = val.unwrap().into_struct_value();

                    // Print opening paren
                    let open_paren = self
                        .builder
                        .build_global_string_ptr("(", "tuple_open")
                        .unwrap();
                    let fmt_s = self
                        .builder
                        .build_global_string_ptr("%s", "fmt_s_tuple")
                        .unwrap();
                    self.builder
                        .build_call(
                            printf_fn,
                            &[
                                fmt_s.as_pointer_value().into(),
                                open_paren.as_pointer_value().into(),
                            ],
                            "",
                        )
                        .unwrap();

                    for (i, et) in elem_types.iter().enumerate() {
                        if i > 0 {
                            let sep = self
                                .builder
                                .build_global_string_ptr(", ", "tuple_sep")
                                .unwrap();
                            self.builder
                                .build_call(
                                    printf_fn,
                                    &[
                                        fmt_s.as_pointer_value().into(),
                                        sep.as_pointer_value().into(),
                                    ],
                                    "",
                                )
                                .unwrap();
                        }

                        let elem_val = self
                            .builder
                            .build_extract_value(tuple_val, i as u32, &format!("tup_el_{}", i))
                            .unwrap();

                        match et {
                            NyType::I32 => {
                                let fmt_d = self
                                    .builder
                                    .build_global_string_ptr("%d", "fmt_d_tup")
                                    .unwrap();
                                self.builder
                                    .build_call(
                                        printf_fn,
                                        &[fmt_d.as_pointer_value().into(), elem_val.into()],
                                        "",
                                    )
                                    .unwrap();
                            }
                            NyType::I64 => {
                                let fmt_ld = self
                                    .builder
                                    .build_global_string_ptr("%ld", "fmt_ld_tup")
                                    .unwrap();
                                self.builder
                                    .build_call(
                                        printf_fn,
                                        &[fmt_ld.as_pointer_value().into(), elem_val.into()],
                                        "",
                                    )
                                    .unwrap();
                            }
                            NyType::F64 | NyType::F32 => {
                                let fmt_f = self
                                    .builder
                                    .build_global_string_ptr("%f", "fmt_f_tup")
                                    .unwrap();
                                let float_val = if *et == NyType::F32 {
                                    let f32_val = elem_val.into_float_value();
                                    self.builder
                                        .build_float_ext(
                                            f32_val,
                                            self.context.f64_type(),
                                            "fpext_tup",
                                        )
                                        .unwrap()
                                        .into()
                                } else {
                                    elem_val
                                };
                                self.builder
                                    .build_call(
                                        printf_fn,
                                        &[fmt_f.as_pointer_value().into(), float_val.into()],
                                        "",
                                    )
                                    .unwrap();
                            }
                            NyType::Bool => {
                                let bool_v = elem_val.into_int_value();
                                let ts = self
                                    .builder
                                    .build_global_string_ptr("true", "t_tup")
                                    .unwrap();
                                let fs = self
                                    .builder
                                    .build_global_string_ptr("false", "f_tup")
                                    .unwrap();
                                let sel = self
                                    .builder
                                    .build_select(
                                        bool_v,
                                        ts.as_pointer_value(),
                                        fs.as_pointer_value(),
                                        "bs_tup",
                                    )
                                    .unwrap();
                                self.builder
                                    .build_call(
                                        printf_fn,
                                        &[fmt_s.as_pointer_value().into(), sel.into()],
                                        "",
                                    )
                                    .unwrap();
                            }
                            NyType::Str => {
                                let write_fn = self.get_or_declare_write();
                                let sv = elem_val.into_struct_value();
                                let ptr = self
                                    .builder
                                    .build_extract_value(sv, 0, "tup_str_ptr")
                                    .unwrap();
                                let len = self
                                    .builder
                                    .build_extract_value(sv, 1, "tup_str_len")
                                    .unwrap();
                                let fd = self.context.i32_type().const_int(1, false);
                                self.builder
                                    .build_call(write_fn, &[fd.into(), ptr.into(), len.into()], "")
                                    .unwrap();
                            }
                            _ => {
                                let fmt_d = self
                                    .builder
                                    .build_global_string_ptr("%d", "fmt_d_tup_fb")
                                    .unwrap();
                                self.builder
                                    .build_call(
                                        printf_fn,
                                        &[fmt_d.as_pointer_value().into(), elem_val.into()],
                                        "",
                                    )
                                    .unwrap();
                            }
                        }
                    }

                    // Print closing paren
                    let close_paren = self
                        .builder
                        .build_global_string_ptr(")", "tuple_close")
                        .unwrap();
                    self.builder
                        .build_call(
                            printf_fn,
                            &[
                                fmt_s.as_pointer_value().into(),
                                close_paren.as_pointer_value().into(),
                            ],
                            "",
                        )
                        .unwrap();
                }
                NyType::Vec(ref elem_ty) => {
                    // Print Vec as [elem1, elem2, ...]
                    let printf_fn = self.get_or_declare_printf();
                    let vec_val = val.unwrap().into_struct_value();
                    let data_ptr = self
                        .builder
                        .build_extract_value(vec_val, 0, "vec_data")
                        .unwrap()
                        .into_pointer_value();
                    let len = self
                        .builder
                        .build_extract_value(vec_val, 1, "vec_len")
                        .unwrap()
                        .into_int_value();

                    let open = self
                        .builder
                        .build_global_string_ptr("[", "vec_open")
                        .unwrap();
                    let fmt_s = self
                        .builder
                        .build_global_string_ptr("%s", "fmt_s_vec")
                        .unwrap();
                    self.builder
                        .build_call(
                            printf_fn,
                            &[fmt_s.as_pointer_value().into(), open.as_pointer_value().into()],
                            "",
                        )
                        .unwrap();

                    // Loop to print each element
                    let i_alloca = self
                        .builder
                        .build_alloca(self.context.i64_type(), "vec_print_i")
                        .unwrap();
                    self.builder
                        .build_store(i_alloca, self.context.i64_type().const_zero())
                        .unwrap();

                    let cond_bb = self
                        .context
                        .append_basic_block(*function, "vec_print_cond");
                    let body_bb = self
                        .context
                        .append_basic_block(*function, "vec_print_body");
                    let done_bb = self
                        .context
                        .append_basic_block(*function, "vec_print_done");

                    self.builder.build_unconditional_branch(cond_bb).unwrap();

                    self.builder.position_at_end(cond_bb);
                    let i = self
                        .builder
                        .build_load(self.context.i64_type(), i_alloca, "i")
                        .unwrap()
                        .into_int_value();
                    let cmp = self
                        .builder
                        .build_int_compare(IntPredicate::ULT, i, len, "vec_cmp")
                        .unwrap();
                    self.builder
                        .build_conditional_branch(cmp, body_bb, done_bb)
                        .unwrap();

                    self.builder.position_at_end(body_bb);
                    // Print separator
                    let zero = self.context.i64_type().const_zero();
                    let not_first = self
                        .builder
                        .build_int_compare(IntPredicate::UGT, i, zero, "not_first")
                        .unwrap();
                    let sep = self
                        .builder
                        .build_global_string_ptr(", ", "vec_sep")
                        .unwrap();
                    let empty = self
                        .builder
                        .build_global_string_ptr("", "vec_empty")
                        .unwrap();
                    let sep_str = self
                        .builder
                        .build_select(
                            not_first,
                            sep.as_pointer_value(),
                            empty.as_pointer_value(),
                            "sep_sel",
                        )
                        .unwrap();
                    self.builder
                        .build_call(printf_fn, &[fmt_s.as_pointer_value().into(), sep_str.into()], "")
                        .unwrap();

                    // Print element
                    let elem_llvm = ny_to_llvm(self.context, elem_ty);
                    let elem_ptr = unsafe {
                        self.builder
                            .build_in_bounds_gep(elem_llvm, data_ptr, &[i], "vec_ep")
                            .unwrap()
                    };
                    let elem_val = self
                        .builder
                        .build_load(elem_llvm, elem_ptr, "vec_e")
                        .unwrap();
                    let fmt_d = self
                        .builder
                        .build_global_string_ptr("%d", "fmt_d_vec")
                        .unwrap();
                    self.builder
                        .build_call(
                            printf_fn,
                            &[fmt_d.as_pointer_value().into(), elem_val.into()],
                            "",
                        )
                        .unwrap();

                    // i++
                    let next_i = self
                        .builder
                        .build_int_add(i, self.context.i64_type().const_int(1, false), "next_i")
                        .unwrap();
                    self.builder.build_store(i_alloca, next_i).unwrap();
                    self.builder.build_unconditional_branch(cond_bb).unwrap();

                    self.builder.position_at_end(done_bb);
                    let close = self
                        .builder
                        .build_global_string_ptr("]", "vec_close")
                        .unwrap();
                    self.builder
                        .build_call(
                            printf_fn,
                            &[fmt_s.as_pointer_value().into(), close.as_pointer_value().into()],
                            "",
                        )
                        .unwrap();
                }
                _ => {
                    // Fallback: try to print as i32
                    if let Some(v) = val {
                        let printf_fn = self.get_or_declare_printf();
                        let fmt = self
                            .builder
                            .build_global_string_ptr("%d", "fmt_fallback")
                            .unwrap();
                        self.builder
                            .build_call(printf_fn, &[fmt.as_pointer_value().into(), v.into()], "")
                            .unwrap();
                    }
                }
            }
        }

        // For println, print a trailing newline
        if is_println {
            let printf_fn = self.get_or_declare_printf();
            let newline = self
                .builder
                .build_global_string_ptr("\n", "newline")
                .unwrap();
            let fmt_s = self
                .builder
                .build_global_string_ptr("%s", "fmt_nl")
                .unwrap();
            self.builder
                .build_call(
                    printf_fn,
                    &[
                        fmt_s.as_pointer_value().into(),
                        newline.as_pointer_value().into(),
                    ],
                    "",
                )
                .unwrap();
        }

        // Flush stdout after every print/println to avoid buffering issues
        let fflush_fn = self.get_or_declare_fflush();
        let null_ptr = self.context.ptr_type(AddressSpace::default()).const_null();
        self.builder
            .build_call(fflush_fn, &[null_ptr.into()], "")
            .unwrap();

        Ok(())
    }

    fn get_or_declare_pthread_create(&self) -> FunctionValue<'ctx> {
        if let Some(f) = self.module.get_function("pthread_create") { return f; }
        let ptr_ty = self.context.ptr_type(AddressSpace::default());
        let i32_ty = self.context.i32_type();
        let fn_ty = i32_ty.fn_type(&[ptr_ty.into(), ptr_ty.into(), ptr_ty.into(), ptr_ty.into()], false);
        self.module.add_function("pthread_create", fn_ty, None)
    }

    fn get_or_declare_pthread_join(&self) -> FunctionValue<'ctx> {
        if let Some(f) = self.module.get_function("pthread_join") { return f; }
        let i64_ty = self.context.i64_type();
        let ptr_ty = self.context.ptr_type(AddressSpace::default());
        let fn_ty = self.context.i32_type().fn_type(&[i64_ty.into(), ptr_ty.into()], false);
        self.module.add_function("pthread_join", fn_ty, None)
    }

    fn get_or_declare_ny_arena_new(&self) -> FunctionValue<'ctx> {
        if let Some(f) = self.module.get_function("ny_arena_new") { return f; }
        let ptr_ty = self.context.ptr_type(AddressSpace::default());
        let i64_ty = self.context.i64_type();
        self.module.add_function("ny_arena_new", ptr_ty.fn_type(&[i64_ty.into()], false), None)
    }

    fn get_or_declare_ny_arena_alloc(&self) -> FunctionValue<'ctx> {
        if let Some(f) = self.module.get_function("ny_arena_alloc") { return f; }
        let ptr_ty = self.context.ptr_type(AddressSpace::default());
        let i64_ty = self.context.i64_type();
        self.module.add_function("ny_arena_alloc", ptr_ty.fn_type(&[ptr_ty.into(), i64_ty.into()], false), None)
    }

    fn get_or_declare_ny_arena_free(&self) -> FunctionValue<'ctx> {
        if let Some(f) = self.module.get_function("ny_arena_free") { return f; }
        let ptr_ty = self.context.ptr_type(AddressSpace::default());
        self.module.add_function("ny_arena_free", self.context.void_type().fn_type(&[ptr_ty.into()], false), None)
    }

    fn get_or_declare_ny_arena_reset(&self) -> FunctionValue<'ctx> {
        if let Some(f) = self.module.get_function("ny_arena_reset") { return f; }
        let ptr_ty = self.context.ptr_type(AddressSpace::default());
        self.module.add_function("ny_arena_reset", self.context.void_type().fn_type(&[ptr_ty.into()], false), None)
    }

    fn get_or_declare_ny_arena_bytes_used(&self) -> FunctionValue<'ctx> {
        if let Some(f) = self.module.get_function("ny_arena_bytes_used") { return f; }
        let ptr_ty = self.context.ptr_type(AddressSpace::default());
        let i64_ty = self.context.i64_type();
        self.module.add_function("ny_arena_bytes_used", i64_ty.fn_type(&[ptr_ty.into()], false), None)
    }

    fn get_or_declare_ny_map_new(&self) -> FunctionValue<'ctx> {
        if let Some(f) = self.module.get_function("ny_map_new") {
            return f;
        }
        let ptr_ty = self.context.ptr_type(AddressSpace::default());
        let fn_ty = ptr_ty.fn_type(&[], false);
        self.module.add_function("ny_map_new", fn_ty, None)
    }

    fn get_or_declare_ny_map_insert(&self) -> FunctionValue<'ctx> {
        if let Some(f) = self.module.get_function("ny_map_insert") {
            return f;
        }
        let ptr_ty = self.context.ptr_type(AddressSpace::default());
        let i64_ty = self.context.i64_type();
        let void_ty = self.context.void_type();
        let fn_ty = void_ty.fn_type(
            &[ptr_ty.into(), ptr_ty.into(), i64_ty.into(), i64_ty.into()],
            false,
        );
        self.module.add_function("ny_map_insert", fn_ty, None)
    }

    fn get_or_declare_ny_map_get(&self) -> FunctionValue<'ctx> {
        if let Some(f) = self.module.get_function("ny_map_get") {
            return f;
        }
        let ptr_ty = self.context.ptr_type(AddressSpace::default());
        let i64_ty = self.context.i64_type();
        let fn_ty = i64_ty.fn_type(&[ptr_ty.into(), ptr_ty.into(), i64_ty.into()], false);
        self.module.add_function("ny_map_get", fn_ty, None)
    }

    fn get_or_declare_ny_map_contains(&self) -> FunctionValue<'ctx> {
        if let Some(f) = self.module.get_function("ny_map_contains") {
            return f;
        }
        let ptr_ty = self.context.ptr_type(AddressSpace::default());
        let i64_ty = self.context.i64_type();
        let i32_ty = self.context.i32_type();
        let fn_ty = i32_ty.fn_type(&[ptr_ty.into(), ptr_ty.into(), i64_ty.into()], false);
        self.module.add_function("ny_map_contains", fn_ty, None)
    }

    fn get_or_declare_ny_map_len(&self) -> FunctionValue<'ctx> {
        if let Some(f) = self.module.get_function("ny_map_len") {
            return f;
        }
        let ptr_ty = self.context.ptr_type(AddressSpace::default());
        let i64_ty = self.context.i64_type();
        let fn_ty = i64_ty.fn_type(&[ptr_ty.into()], false);
        self.module.add_function("ny_map_len", fn_ty, None)
    }

    fn get_or_declare_fflush(&self) -> FunctionValue<'ctx> {
        if let Some(f) = self.module.get_function("fflush") {
            return f;
        }
        let ptr_ty = self.context.ptr_type(AddressSpace::default());
        let i32_ty = self.context.i32_type();
        let fflush_ty = i32_ty.fn_type(&[ptr_ty.into()], false);
        self.module.add_function("fflush", fflush_ty, None)
    }

    // ------------------------------------------------------------------
    // Libc function declarations (lazy, idempotent)
    // ------------------------------------------------------------------

    fn get_or_declare_printf(&self) -> FunctionValue<'ctx> {
        if let Some(f) = self.module.get_function("printf") {
            return f;
        }
        let ptr_ty = self.context.ptr_type(AddressSpace::default());
        let printf_ty = self.context.i32_type().fn_type(&[ptr_ty.into()], true); // variadic
        self.module.add_function("printf", printf_ty, None)
    }

    fn get_or_declare_write(&self) -> FunctionValue<'ctx> {
        if let Some(f) = self.module.get_function("write") {
            return f;
        }
        let i32_ty = self.context.i32_type();
        let i64_ty = self.context.i64_type();
        let ptr_ty = self.context.ptr_type(AddressSpace::default());
        // ssize_t write(int fd, const void *buf, size_t count)
        let write_ty = i64_ty.fn_type(&[i32_ty.into(), ptr_ty.into(), i64_ty.into()], false);
        self.module.add_function("write", write_ty, None)
    }

    fn get_or_declare_abort(&self) -> FunctionValue<'ctx> {
        if let Some(f) = self.module.get_function("abort") {
            return f;
        }
        let abort_ty = self.context.void_type().fn_type(&[], false);
        self.module.add_function("abort", abort_ty, None)
    }

    fn get_or_declare_malloc(&self) -> FunctionValue<'ctx> {
        if let Some(f) = self.module.get_function("malloc") {
            return f;
        }
        let ptr_ty = self.context.ptr_type(AddressSpace::default());
        let i64_ty = self.context.i64_type();
        let malloc_ty = ptr_ty.fn_type(&[i64_ty.into()], false);
        self.module.add_function("malloc", malloc_ty, None)
    }

    fn get_or_declare_free(&self) -> FunctionValue<'ctx> {
        if let Some(f) = self.module.get_function("free") {
            return f;
        }
        let ptr_ty = self.context.ptr_type(AddressSpace::default());
        let free_ty = self.context.void_type().fn_type(&[ptr_ty.into()], false);
        self.module.add_function("free", free_ty, None)
    }

    fn get_or_declare_realloc(&self) -> FunctionValue<'ctx> {
        if let Some(f) = self.module.get_function("realloc") {
            return f;
        }
        let ptr_ty = self.context.ptr_type(AddressSpace::default());
        let i64_ty = self.context.i64_type();
        let realloc_ty = ptr_ty.fn_type(&[ptr_ty.into(), i64_ty.into()], false);
        self.module.add_function("realloc", realloc_ty, None)
    }

    fn get_or_declare_memcpy(&self) -> FunctionValue<'ctx> {
        if let Some(f) = self.module.get_function("memcpy") {
            return f;
        }
        let ptr_ty = self.context.ptr_type(AddressSpace::default());
        let i64_ty = self.context.i64_type();
        // void *memcpy(void *dest, const void *src, size_t n)
        let memcpy_ty = ptr_ty.fn_type(&[ptr_ty.into(), ptr_ty.into(), i64_ty.into()], false);
        self.module.add_function("memcpy", memcpy_ty, None)
    }

    fn get_or_declare_fopen(&self) -> FunctionValue<'ctx> {
        if let Some(f) = self.module.get_function("fopen") {
            return f;
        }
        let ptr_ty = self.context.ptr_type(AddressSpace::default());
        let fopen_ty = ptr_ty.fn_type(&[ptr_ty.into(), ptr_ty.into()], false);
        self.module.add_function("fopen", fopen_ty, None)
    }

    fn get_or_declare_fclose(&self) -> FunctionValue<'ctx> {
        if let Some(f) = self.module.get_function("fclose") {
            return f;
        }
        let ptr_ty = self.context.ptr_type(AddressSpace::default());
        let i32_ty = self.context.i32_type();
        let fclose_ty = i32_ty.fn_type(&[ptr_ty.into()], false);
        self.module.add_function("fclose", fclose_ty, None)
    }

    fn get_or_declare_fwrite(&self) -> FunctionValue<'ctx> {
        if let Some(f) = self.module.get_function("fwrite") {
            return f;
        }
        let ptr_ty = self.context.ptr_type(AddressSpace::default());
        let i64_ty = self.context.i64_type();
        // size_t fwrite(const void *ptr, size_t size, size_t nmemb, FILE *stream)
        let fwrite_ty =
            i64_ty.fn_type(&[ptr_ty.into(), i64_ty.into(), i64_ty.into(), ptr_ty.into()], false);
        self.module.add_function("fwrite", fwrite_ty, None)
    }

    fn get_or_declare_fgetc(&self) -> FunctionValue<'ctx> {
        if let Some(f) = self.module.get_function("fgetc") {
            return f;
        }
        let ptr_ty = self.context.ptr_type(AddressSpace::default());
        let i32_ty = self.context.i32_type();
        let fgetc_ty = i32_ty.fn_type(&[ptr_ty.into()], false);
        self.module.add_function("fgetc", fgetc_ty, None)
    }

    fn get_or_declare_exit(&self) -> FunctionValue<'ctx> {
        if let Some(f) = self.module.get_function("exit") {
            return f;
        }
        let i32_ty = self.context.i32_type();
        let exit_ty = self.context.void_type().fn_type(&[i32_ty.into()], false);
        self.module.add_function("exit", exit_ty, None)
    }

    fn get_or_declare_fgets(&self) -> FunctionValue<'ctx> {
        if let Some(f) = self.module.get_function("fgets") {
            return f;
        }
        let ptr_ty = self.context.ptr_type(AddressSpace::default());
        let i32_ty = self.context.i32_type();
        let fgets_ty = ptr_ty.fn_type(&[ptr_ty.into(), i32_ty.into(), ptr_ty.into()], false);
        self.module.add_function("fgets", fgets_ty, None)
    }

    fn get_or_declare_stdin(&self) -> inkwell::values::GlobalValue<'ctx> {
        if let Some(g) = self.module.get_global("stdin") {
            return g;
        }
        let ptr_ty = self.context.ptr_type(AddressSpace::default());
        self.module.add_global(ptr_ty, None, "stdin")
    }

    fn get_or_declare_strlen(&self) -> FunctionValue<'ctx> {
        if let Some(f) = self.module.get_function("strlen") {
            return f;
        }
        let ptr_ty = self.context.ptr_type(AddressSpace::default());
        let i64_ty = self.context.i64_type();
        let strlen_ty = i64_ty.fn_type(&[ptr_ty.into()], false);
        self.module.add_function("strlen", strlen_ty, None)
    }

    fn get_or_declare_atoi(&self) -> FunctionValue<'ctx> {
        if let Some(f) = self.module.get_function("atoi") {
            return f;
        }
        let ptr_ty = self.context.ptr_type(AddressSpace::default());
        let i32_ty = self.context.i32_type();
        let atoi_ty = i32_ty.fn_type(&[ptr_ty.into()], false);
        self.module.add_function("atoi", atoi_ty, None)
    }

    fn get_or_declare_snprintf(&self) -> FunctionValue<'ctx> {
        if let Some(f) = self.module.get_function("snprintf") {
            return f;
        }
        let ptr_ty = self.context.ptr_type(AddressSpace::default());
        let i32_ty = self.context.i32_type();
        let i64_ty = self.context.i64_type();
        let snprintf_ty =
            i32_ty.fn_type(&[ptr_ty.into(), i64_ty.into(), ptr_ty.into()], true);
        self.module.add_function("snprintf", snprintf_ty, None)
    }

    fn get_or_declare_usleep(&self) -> FunctionValue<'ctx> {
        if let Some(f) = self.module.get_function("usleep") {
            return f;
        }
        let i32_ty = self.context.i32_type();
        let usleep_ty = i32_ty.fn_type(&[i32_ty.into()], false);
        self.module.add_function("usleep", usleep_ty, None)
    }

    fn get_or_declare_memcmp(&self) -> FunctionValue<'ctx> {
        if let Some(f) = self.module.get_function("memcmp") {
            return f;
        }
        let ptr_ty = self.context.ptr_type(AddressSpace::default());
        let i32_ty = self.context.i32_type();
        let i64_ty = self.context.i64_type();
        // int memcmp(const void *s1, const void *s2, size_t n)
        let memcmp_ty = i32_ty.fn_type(&[ptr_ty.into(), ptr_ty.into(), i64_ty.into()], false);
        self.module.add_function("memcmp", memcmp_ty, None)
    }

    // ------------------------------------------------------------------
    // Binary and unary operations
    // ------------------------------------------------------------------

    fn compile_binop(
        &self,
        op: BinOp,
        lhs: BasicValueEnum<'ctx>,
        rhs: BasicValueEnum<'ctx>,
    ) -> Result<BasicValueEnum<'ctx>, Vec<CompileError>> {
        // Handle string operations (both operands are {ptr, len} struct values)
        if lhs.is_struct_value() && rhs.is_struct_value() {
            let l = lhs.into_struct_value();
            let r = rhs.into_struct_value();

            match op {
                BinOp::Add => {
                    // String concatenation
                    let l_ptr = self.builder.build_extract_value(l, 0, "l_ptr").unwrap();
                    let l_len = self
                        .builder
                        .build_extract_value(l, 1, "l_len")
                        .unwrap()
                        .into_int_value();
                    let r_ptr = self.builder.build_extract_value(r, 0, "r_ptr").unwrap();
                    let r_len = self
                        .builder
                        .build_extract_value(r, 1, "r_len")
                        .unwrap()
                        .into_int_value();

                    let total_len = self
                        .builder
                        .build_int_add(l_len, r_len, "total_len")
                        .unwrap();

                    // malloc(total_len)
                    let malloc_fn = self.get_or_declare_malloc();
                    let result_ptr = self
                        .builder
                        .build_call(malloc_fn, &[total_len.into()], "concat_buf")
                        .unwrap()
                        .try_as_basic_value()
                        .basic()
                        .unwrap()
                        .into_pointer_value();

                    // memcpy(result_ptr, l_ptr, l_len)
                    let memcpy_fn = self.get_or_declare_memcpy();
                    self.builder
                        .build_call(
                            memcpy_fn,
                            &[result_ptr.into(), l_ptr.into(), l_len.into()],
                            "",
                        )
                        .unwrap();

                    // memcpy(result_ptr + l_len, r_ptr, r_len)
                    let i8_ty = self.context.i8_type();
                    let dest_offset = unsafe {
                        self.builder
                            .build_in_bounds_gep(i8_ty, result_ptr, &[l_len], "concat_dest")
                            .unwrap()
                    };
                    self.builder
                        .build_call(
                            memcpy_fn,
                            &[dest_offset.into(), r_ptr.into(), r_len.into()],
                            "",
                        )
                        .unwrap();

                    // Build {result_ptr, total_len} struct
                    let str_ty = str_type(self.context);
                    let str_val = str_ty.const_zero();
                    let str_val = self
                        .builder
                        .build_insert_value(str_val, result_ptr, 0, "cat_ptr")
                        .unwrap();
                    let str_val = self
                        .builder
                        .build_insert_value(str_val, total_len, 1, "cat_len")
                        .unwrap();
                    return Ok(str_val.into_struct_value().into());
                }
                BinOp::Eq | BinOp::Ne => {
                    // String comparison
                    let l_ptr = self.builder.build_extract_value(l, 0, "l_ptr").unwrap();
                    let l_len = self
                        .builder
                        .build_extract_value(l, 1, "l_len")
                        .unwrap()
                        .into_int_value();
                    let r_ptr = self.builder.build_extract_value(r, 0, "r_ptr").unwrap();
                    let r_len = self
                        .builder
                        .build_extract_value(r, 1, "r_len")
                        .unwrap()
                        .into_int_value();

                    // Compare lengths
                    let len_eq = self
                        .builder
                        .build_int_compare(IntPredicate::EQ, l_len, r_len, "len_eq")
                        .unwrap();

                    // If lengths are equal, compare contents with memcmp
                    let memcmp_fn = self.get_or_declare_memcmp();
                    let cmp_result = self
                        .builder
                        .build_call(
                            memcmp_fn,
                            &[l_ptr.into(), r_ptr.into(), l_len.into()],
                            "memcmp_res",
                        )
                        .unwrap()
                        .try_as_basic_value()
                        .basic()
                        .unwrap()
                        .into_int_value();
                    let cmp_eq = self
                        .builder
                        .build_int_compare(
                            IntPredicate::EQ,
                            cmp_result,
                            self.context.i32_type().const_zero(),
                            "cmp_eq",
                        )
                        .unwrap();

                    // Final result: lengths equal AND contents equal
                    let str_eq = self.builder.build_and(len_eq, cmp_eq, "str_eq").unwrap();

                    if op == BinOp::Ne {
                        let negated = self.builder.build_not(str_eq, "str_ne").unwrap();
                        return Ok(negated.into());
                    }
                    return Ok(str_eq.into());
                }
                _ => {
                    return Err(vec![CompileError::type_error(
                        "unsupported binary operation on struct/string types".to_string(),
                        Span::empty(0),
                    )]);
                }
            }
        }

        // Pointer arithmetic: ptr + int or ptr - int
        if lhs.is_pointer_value() && rhs.is_int_value() {
            let ptr = lhs.into_pointer_value();
            let offset = rhs.into_int_value();
            let i8_ty = self.context.i8_type();
            match op {
                BinOp::Add => {
                    let offset_i64 = self
                        .builder
                        .build_int_s_extend_or_bit_cast(
                            offset,
                            self.context.i64_type(),
                            "ptr_off",
                        )
                        .unwrap();
                    let result = unsafe {
                        self.builder
                            .build_in_bounds_gep(i8_ty, ptr, &[offset_i64], "ptr_add")
                            .unwrap()
                    };
                    return Ok(result.into());
                }
                BinOp::Sub => {
                    let neg_offset = self
                        .builder
                        .build_int_neg(offset, "neg_off")
                        .unwrap();
                    let neg_i64 = self
                        .builder
                        .build_int_s_extend_or_bit_cast(
                            neg_offset,
                            self.context.i64_type(),
                            "ptr_neg",
                        )
                        .unwrap();
                    let result = unsafe {
                        self.builder
                            .build_in_bounds_gep(i8_ty, ptr, &[neg_i64], "ptr_sub")
                            .unwrap()
                    };
                    return Ok(result.into());
                }
                _ => {
                    return Err(vec![CompileError::type_error(
                        "only + and - are supported for pointer arithmetic".to_string(),
                        Span::empty(0),
                    )]);
                }
            }
        }

        if lhs.is_int_value() && rhs.is_int_value() {
            let l = lhs.into_int_value();
            let r = rhs.into_int_value();
            let result: BasicValueEnum = match op {
                BinOp::Add => self.builder.build_int_add(l, r, "add").unwrap().into(),
                BinOp::Sub => self.builder.build_int_sub(l, r, "sub").unwrap().into(),
                BinOp::Mul => self.builder.build_int_mul(l, r, "mul").unwrap().into(),
                BinOp::Div => self
                    .builder
                    .build_int_signed_div(l, r, "div")
                    .unwrap()
                    .into(),
                BinOp::Mod => self
                    .builder
                    .build_int_signed_rem(l, r, "rem")
                    .unwrap()
                    .into(),
                BinOp::Eq => self
                    .builder
                    .build_int_compare(IntPredicate::EQ, l, r, "eq")
                    .unwrap()
                    .into(),
                BinOp::Ne => self
                    .builder
                    .build_int_compare(IntPredicate::NE, l, r, "ne")
                    .unwrap()
                    .into(),
                BinOp::Lt => self
                    .builder
                    .build_int_compare(IntPredicate::SLT, l, r, "lt")
                    .unwrap()
                    .into(),
                BinOp::Gt => self
                    .builder
                    .build_int_compare(IntPredicate::SGT, l, r, "gt")
                    .unwrap()
                    .into(),
                BinOp::Le => self
                    .builder
                    .build_int_compare(IntPredicate::SLE, l, r, "le")
                    .unwrap()
                    .into(),
                BinOp::Ge => self
                    .builder
                    .build_int_compare(IntPredicate::SGE, l, r, "ge")
                    .unwrap()
                    .into(),
                BinOp::And => self.builder.build_and(l, r, "and").unwrap().into(),
                BinOp::Or => self.builder.build_or(l, r, "or").unwrap().into(),
                BinOp::BitAnd => self.builder.build_and(l, r, "bitand").unwrap().into(),
                BinOp::BitOr => self.builder.build_or(l, r, "bitor").unwrap().into(),
                BinOp::BitXor => self.builder.build_xor(l, r, "bitxor").unwrap().into(),
                BinOp::Shl => self.builder.build_left_shift(l, r, "shl").unwrap().into(),
                BinOp::Shr => self
                    .builder
                    .build_right_shift(l, r, true, "shr")
                    .unwrap()
                    .into(),
            };
            Ok(result)
        } else if lhs.is_float_value() && rhs.is_float_value() {
            let l = lhs.into_float_value();
            let r = rhs.into_float_value();
            let result: BasicValueEnum = match op {
                BinOp::Add => self.builder.build_float_add(l, r, "fadd").unwrap().into(),
                BinOp::Sub => self.builder.build_float_sub(l, r, "fsub").unwrap().into(),
                BinOp::Mul => self.builder.build_float_mul(l, r, "fmul").unwrap().into(),
                BinOp::Div => self.builder.build_float_div(l, r, "fdiv").unwrap().into(),
                BinOp::Mod => self.builder.build_float_rem(l, r, "frem").unwrap().into(),
                BinOp::Eq => self
                    .builder
                    .build_float_compare(FloatPredicate::OEQ, l, r, "feq")
                    .unwrap()
                    .into(),
                BinOp::Ne => self
                    .builder
                    .build_float_compare(FloatPredicate::ONE, l, r, "fne")
                    .unwrap()
                    .into(),
                BinOp::Lt => self
                    .builder
                    .build_float_compare(FloatPredicate::OLT, l, r, "flt")
                    .unwrap()
                    .into(),
                BinOp::Gt => self
                    .builder
                    .build_float_compare(FloatPredicate::OGT, l, r, "fgt")
                    .unwrap()
                    .into(),
                BinOp::Le => self
                    .builder
                    .build_float_compare(FloatPredicate::OLE, l, r, "fle")
                    .unwrap()
                    .into(),
                BinOp::Ge => self
                    .builder
                    .build_float_compare(FloatPredicate::OGE, l, r, "fge")
                    .unwrap()
                    .into(),
                BinOp::And
                | BinOp::Or
                | BinOp::BitAnd
                | BinOp::BitOr
                | BinOp::BitXor
                | BinOp::Shl
                | BinOp::Shr => {
                    unreachable!("logical/bitwise ops on floats")
                }
            };
            Ok(result)
        } else if lhs.is_vector_value() && rhs.is_vector_value() {
            // SIMD vector arithmetic
            let l = lhs.into_vector_value();
            let r = rhs.into_vector_value();
            // Check if the element type is float or int
            let elem_kind = l.get_type().get_element_type();
            if elem_kind.is_float_type() {
                let result: BasicValueEnum = match op {
                    BinOp::Add => self.builder.build_float_add(l, r, "vadd").unwrap().into(),
                    BinOp::Sub => self.builder.build_float_sub(l, r, "vsub").unwrap().into(),
                    BinOp::Mul => self.builder.build_float_mul(l, r, "vmul").unwrap().into(),
                    BinOp::Div => self.builder.build_float_div(l, r, "vdiv").unwrap().into(),
                    _ => {
                        return Err(vec![CompileError::type_error(
                            "unsupported SIMD operation".to_string(),
                            Span::empty(0),
                        )]);
                    }
                };
                Ok(result)
            } else {
                let result: BasicValueEnum = match op {
                    BinOp::Add => self.builder.build_int_add(l, r, "vadd").unwrap().into(),
                    BinOp::Sub => self.builder.build_int_sub(l, r, "vsub").unwrap().into(),
                    BinOp::Mul => self.builder.build_int_mul(l, r, "vmul").unwrap().into(),
                    _ => {
                        return Err(vec![CompileError::type_error(
                            "unsupported SIMD operation".to_string(),
                            Span::empty(0),
                        )]);
                    }
                };
                Ok(result)
            }
        } else {
            Err(vec![CompileError::type_error(
                "binary operation on incompatible types".to_string(),
                Span::empty(0),
            )])
        }
    }

    fn compile_unaryop(
        &self,
        op: UnaryOp,
        operand: BasicValueEnum<'ctx>,
    ) -> Result<BasicValueEnum<'ctx>, Vec<CompileError>> {
        match op {
            UnaryOp::Neg => {
                if operand.is_int_value() {
                    Ok(self
                        .builder
                        .build_int_neg(operand.into_int_value(), "neg")
                        .unwrap()
                        .into())
                } else {
                    Ok(self
                        .builder
                        .build_float_neg(operand.into_float_value(), "fneg")
                        .unwrap()
                        .into())
                }
            }
            UnaryOp::Not => Ok(self
                .builder
                .build_not(operand.into_int_value(), "not")
                .unwrap()
                .into()),
            UnaryOp::BitNot => Ok(self
                .builder
                .build_not(operand.into_int_value(), "bitnot")
                .unwrap()
                .into()),
        }
    }
}
