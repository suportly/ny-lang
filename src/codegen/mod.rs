pub mod types;

use std::collections::HashMap;
use std::path::Path;
use std::process::Command;

use inkwell::builder::Builder;
use inkwell::context::Context;
use inkwell::module::Module;
use inkwell::passes::PassBuilderOptions;
use inkwell::targets::{
    CodeModel, FileType, InitializationConfig, RelocMode, Target, TargetMachine,
};
use inkwell::types::{BasicType, BasicTypeEnum};
use inkwell::values::{BasicValueEnum, FunctionValue, PointerValue};
use inkwell::FloatPredicate;
use inkwell::IntPredicate;
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

    // Module and builder must be dropped before context. By scoping the codegen
    // work inside a block, Rust drops them in the right order.
    let module = context.create_module(source_path.to_str().unwrap_or("main"));
    let builder = context.create_builder();

    let mut codegen = CodeGen {
        context: &context,
        module,
        builder,
        variables: HashMap::new(),
        functions: HashMap::new(),
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

struct CodeGen<'ctx> {
    context: &'ctx Context,
    module: Module<'ctx>,
    builder: Builder<'ctx>,
    variables: HashMap<String, (PointerValue<'ctx>, NyType)>,
    functions: HashMap<String, (FunctionValue<'ctx>, Vec<NyType>, NyType)>,
}

impl<'ctx> CodeGen<'ctx> {
    fn compile_program(&mut self, program: &Program) -> Result<(), Vec<CompileError>> {
        // First pass: declare all functions
        for item in &program.items {
            match item {
                Item::FunctionDef {
                    name,
                    params,
                    return_type,
                    ..
                } => {
                    let ret_ty = NyType::from_name(&return_type.name).unwrap_or(NyType::Unit);
                    let param_types: Vec<NyType> = params
                        .iter()
                        .map(|p| NyType::from_name(&p.ty.name).unwrap_or(NyType::I32))
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
        }

        // Second pass: compile function bodies
        for item in &program.items {
            match item {
                Item::FunctionDef {
                    name, params, body, ..
                } => {
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
        }

        Ok(())
    }

    fn compile_expr(
        &mut self,
        expr: &Expr,
        function: &FunctionValue<'ctx>,
    ) -> Result<Option<BasicValueEnum<'ctx>>, Vec<CompileError>> {
        match expr {
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
            },
            Expr::Ident { name, .. } => {
                if let Some((ptr, ty)) = self.variables.get(name) {
                    let llvm_ty = ny_to_llvm(self.context, ty);
                    let val = self.builder.build_load(llvm_ty, *ptr, name).unwrap();
                    Ok(Some(val))
                } else {
                    Ok(None)
                }
            }
            Expr::BinOp { op, lhs, rhs, .. } => {
                let lhs_val = self.compile_expr(lhs, function)?.unwrap();
                let rhs_val = self.compile_expr(rhs, function)?.unwrap();
                let result = self.compile_binop(*op, lhs_val, rhs_val)?;
                Ok(Some(result))
            }
            Expr::UnaryOp { op, operand, .. } => {
                let val = self.compile_expr(operand, function)?.unwrap();
                let result = self.compile_unaryop(*op, val)?;
                Ok(Some(result))
            }
            Expr::Call { callee, args, .. } => {
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
            Expr::Block {
                stmts, tail_expr, ..
            } => {
                for stmt in stmts {
                    self.compile_stmt(stmt, function)?;
                    // If the block has been terminated (return), stop
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
        }
    }

    fn compile_stmt(
        &mut self,
        stmt: &Stmt,
        function: &FunctionValue<'ctx>,
    ) -> Result<(), Vec<CompileError>> {
        match stmt {
            Stmt::VarDecl { name, ty, init, .. } => {
                let val = self.compile_expr(init, function)?.unwrap();
                let lnge_ty = ty
                    .as_ref()
                    .and_then(|t| NyType::from_name(&t.name))
                    .unwrap_or(NyType::I32);
                let llvm_ty = ny_to_llvm(self.context, &lnge_ty);
                let alloca = self.builder.build_alloca(llvm_ty, name).unwrap();
                self.builder.build_store(alloca, val).unwrap();
                self.variables.insert(name.clone(), (alloca, lnge_ty));
                Ok(())
            }
            Stmt::ConstDecl {
                name, ty, value, ..
            } => {
                let val = self.compile_expr(value, function)?.unwrap();
                let lnge_ty = ty
                    .as_ref()
                    .and_then(|t| NyType::from_name(&t.name))
                    .unwrap_or(NyType::I32);
                let llvm_ty = ny_to_llvm(self.context, &lnge_ty);
                let alloca = self.builder.build_alloca(llvm_ty, name).unwrap();
                self.builder.build_store(alloca, val).unwrap();
                self.variables.insert(name.clone(), (alloca, lnge_ty));
                Ok(())
            }
            Stmt::Assign { target, value, .. } => {
                let val = self.compile_expr(value, function)?.unwrap();
                if let Some((ptr, _)) = self.variables.get(target) {
                    self.builder.build_store(*ptr, val).unwrap();
                }
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
            Stmt::While {
                condition, body, ..
            } => {
                let cond_bb = self.context.append_basic_block(*function, "while_cond");
                let body_bb = self.context.append_basic_block(*function, "while_body");
                let exit_bb = self.context.append_basic_block(*function, "while_exit");

                self.builder.build_unconditional_branch(cond_bb).unwrap();

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

                // Exit
                self.builder.position_at_end(exit_bb);
                Ok(())
            }
        }
    }

    fn compile_binop(
        &self,
        op: BinOp,
        lhs: BasicValueEnum<'ctx>,
        rhs: BasicValueEnum<'ctx>,
    ) -> Result<BasicValueEnum<'ctx>, Vec<CompileError>> {
        // Determine if integer or float based on the values
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
