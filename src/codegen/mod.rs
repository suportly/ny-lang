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
        loop_stack: Vec::new(),
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
    let status = Command::new("cc")
        .arg(obj_path)
        .arg("-o")
        .arg(output_path)
        .arg("-lm")
        .arg("-lc")
        .status()
        .map_err(|e| {
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
    loop_stack: Vec<LoopFrame<'ctx>>,
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
                _ => self.infer_expr_type(lhs),
            },
            Expr::UnaryOp { op, operand, .. } => match op {
                UnaryOp::Not => NyType::Bool,
                UnaryOp::Neg => self.infer_expr_type(operand),
            },
            Expr::Call { callee, .. } => {
                if let Some((_, _, ret_ty)) = self.functions.get(callee) {
                    ret_ty.clone()
                } else {
                    NyType::Unit
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
            Expr::MethodCall { .. } => NyType::Unit,
        }
    }

    // ------------------------------------------------------------------
    // Compile program: three passes (struct types, function decls, bodies)
    // ------------------------------------------------------------------

    fn compile_program(&mut self, program: &Program) -> Result<(), Vec<CompileError>> {
        // Pass 0: Register all LLVM named struct types
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
        }

        // Pass 1: Declare all functions (forward references)
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

        // Pass 2: Compile function bodies
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

                self.compile_expr(body, &function)?;

                // Add return void if no terminator
                let current_block = self.builder.get_insert_block().unwrap();
                if current_block.get_terminator().is_none() {
                    self.builder.build_return(None).unwrap();
                }

                // Restore outer scope
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

                let (func, _, ret_ty) = self.functions[callee].clone();
                let mut arg_values = Vec::new();
                for arg in args {
                    let val = self.compile_expr(arg, function)?.unwrap();
                    arg_values.push(val.into());
                }
                let call = self.builder.build_call(func, &arg_values, "call").unwrap();

                if ret_ty == NyType::Unit {
                    Ok(None)
                } else {
                    Ok(call.try_as_basic_value().basic())
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
                    let obj_ty = self.infer_expr_type(object);
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
                    // Type inference: use the inferred type from the init expression
                    self.infer_expr_type(init)
                };
                let llvm_ty = ny_to_llvm(self.context, &ny_ty);
                let alloca = self.builder.build_alloca(llvm_ty, name).unwrap();
                self.builder.build_store(alloca, val).unwrap();
                self.variables.insert(name.clone(), (alloca, ny_ty));
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
                if let Some(val_expr) = value {
                    let val = self.compile_expr(val_expr, function)?;
                    if let Some(v) = val {
                        self.builder.build_return(Some(&v)).unwrap();
                    } else {
                        self.builder.build_return(None).unwrap();
                    }
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

        // Fail block: call abort
        self.builder.position_at_end(fail_bb);
        let abort_fn = self.get_or_declare_abort();
        self.builder.build_call(abort_fn, &[], "").unwrap();
        self.builder.build_unreachable().unwrap();

        // Continue from ok block
        self.builder.position_at_end(ok_bb);
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

        Ok(())
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

    // ------------------------------------------------------------------
    // Binary and unary operations
    // ------------------------------------------------------------------

    fn compile_binop(
        &self,
        op: BinOp,
        lhs: BasicValueEnum<'ctx>,
        rhs: BasicValueEnum<'ctx>,
    ) -> Result<BasicValueEnum<'ctx>, Vec<CompileError>> {
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
                BinOp::And | BinOp::Or => unreachable!("logical ops on floats"),
            };
            Ok(result)
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
        }
    }
}
