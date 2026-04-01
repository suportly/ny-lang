use std::collections::HashMap;

use crate::common::{CompileError, NyType, Span};
use crate::parser::ast::*;
use crate::semantic::resolver::ResolvedInfo;

pub struct TypeChecker {
    scopes: Vec<HashMap<String, NyType>>,
    functions: HashMap<String, (Vec<NyType>, NyType)>,
    current_return_type: NyType,
    errors: Vec<CompileError>,
}

impl TypeChecker {
    pub fn new(resolved: &ResolvedInfo) -> Self {
        let functions = resolved
            .functions
            .iter()
            .map(|(name, (params, ret, _))| (name.clone(), (params.clone(), ret.clone())))
            .collect();

        Self {
            scopes: vec![HashMap::new()],
            functions,
            current_return_type: NyType::Unit,
            errors: Vec::new(),
        }
    }

    fn push_scope(&mut self) {
        self.scopes.push(HashMap::new());
    }

    fn pop_scope(&mut self) {
        self.scopes.pop();
    }

    fn declare(&mut self, name: &str, ty: NyType) {
        self.scopes.last_mut().unwrap().insert(name.to_string(), ty);
    }

    fn lookup(&self, name: &str) -> Option<NyType> {
        for scope in self.scopes.iter().rev() {
            if let Some(ty) = scope.get(name) {
                return Some(ty.clone());
            }
        }
        None
    }

    pub fn check(program: &Program, resolved: &ResolvedInfo) -> Result<(), Vec<CompileError>> {
        let mut checker = TypeChecker::new(resolved);

        for item in &program.items {
            checker.check_item(item);
        }

        // Verify main function exists and returns i32
        match resolved.functions.get("main") {
            None => {
                checker.errors.push(CompileError::name_error(
                    "no 'main' function found".to_string(),
                    Span::empty(0),
                ));
            }
            Some((params, ret, span)) => {
                if !params.is_empty() {
                    checker.errors.push(CompileError::type_error(
                        "'main' function must have no parameters".to_string(),
                        *span,
                    ));
                }
                if *ret != NyType::I32 {
                    checker.errors.push(CompileError::type_error(
                        format!("'main' function must return i32, found {}", ret),
                        *span,
                    ));
                }
            }
        }

        if checker.errors.is_empty() {
            Ok(())
        } else {
            Err(checker.errors)
        }
    }

    fn check_item(&mut self, item: &Item) {
        match item {
            Item::FunctionDef {
                name: _,
                params,
                return_type,
                body,
                ..
            } => {
                let ret_ty = NyType::from_name(&return_type.name).unwrap_or(NyType::Unit);
                self.current_return_type = ret_ty;

                self.push_scope();
                for p in params {
                    if let Some(ty) = NyType::from_name(&p.ty.name) {
                        self.declare(&p.name, ty);
                    }
                }
                self.check_expr(body);
                self.pop_scope();
            }
        }
    }

    fn check_expr(&mut self, expr: &Expr) -> NyType {
        match expr {
            Expr::Literal { value, .. } => match value {
                LitValue::Int(_) => NyType::I32,   // default integer type
                LitValue::Float(_) => NyType::F64, // default float type
                LitValue::Bool(_) => NyType::Bool,
            },
            Expr::Ident { name, span: _ } => {
                self.lookup(name).unwrap_or({
                    // Already reported by resolver
                    NyType::I32
                })
            }
            Expr::BinOp { op, lhs, rhs, span } => {
                let lhs_ty = self.check_expr(lhs);
                let rhs_ty = self.check_expr(rhs);

                match op {
                    BinOp::Add | BinOp::Sub | BinOp::Mul | BinOp::Div | BinOp::Mod => {
                        if !lhs_ty.is_numeric() {
                            self.errors.push(CompileError::type_error(
                                format!(
                                    "arithmetic operator requires numeric type, found '{}'",
                                    lhs_ty
                                ),
                                lhs.span(),
                            ));
                        }
                        if lhs_ty != rhs_ty {
                            self.errors.push(CompileError::type_error(
                                format!(
                                    "type mismatch in arithmetic: '{}' and '{}'",
                                    lhs_ty, rhs_ty
                                ),
                                *span,
                            ));
                        }
                        lhs_ty
                    }
                    BinOp::Eq | BinOp::Ne | BinOp::Lt | BinOp::Gt | BinOp::Le | BinOp::Ge => {
                        if lhs_ty != rhs_ty {
                            self.errors.push(CompileError::type_error(
                                format!("cannot compare '{}' with '{}'", lhs_ty, rhs_ty),
                                *span,
                            ));
                        }
                        NyType::Bool
                    }
                    BinOp::And | BinOp::Or => {
                        if lhs_ty != NyType::Bool {
                            self.errors.push(CompileError::type_error(
                                format!("logical operator requires bool, found '{}'", lhs_ty),
                                lhs.span(),
                            ));
                        }
                        if rhs_ty != NyType::Bool {
                            self.errors.push(CompileError::type_error(
                                format!("logical operator requires bool, found '{}'", rhs_ty),
                                rhs.span(),
                            ));
                        }
                        NyType::Bool
                    }
                }
            }
            Expr::UnaryOp { op, operand, span } => {
                let operand_ty = self.check_expr(operand);
                match op {
                    UnaryOp::Neg => {
                        if !operand_ty.is_numeric() {
                            self.errors.push(CompileError::type_error(
                                format!("negation requires numeric type, found '{}'", operand_ty),
                                *span,
                            ));
                        }
                        operand_ty
                    }
                    UnaryOp::Not => {
                        if operand_ty != NyType::Bool {
                            self.errors.push(CompileError::type_error(
                                format!("logical not requires bool, found '{}'", operand_ty),
                                *span,
                            ));
                        }
                        NyType::Bool
                    }
                }
            }
            Expr::Call { callee, args, span } => {
                if let Some((param_types, ret_type)) = self.functions.get(callee).cloned() {
                    if args.len() != param_types.len() {
                        self.errors.push(CompileError::type_error(
                            format!(
                                "function '{}' expects {} arguments, found {}",
                                callee,
                                param_types.len(),
                                args.len()
                            ),
                            *span,
                        ));
                    } else {
                        for (i, (arg, expected_ty)) in
                            args.iter().zip(param_types.iter()).enumerate()
                        {
                            let arg_ty = self.check_expr(arg);
                            if arg_ty != *expected_ty {
                                self.errors.push(CompileError::type_error(
                                    format!(
                                        "argument {} of '{}': expected '{}', found '{}'",
                                        i + 1,
                                        callee,
                                        expected_ty,
                                        arg_ty
                                    ),
                                    arg.span(),
                                ));
                            }
                        }
                    }
                    ret_type
                } else {
                    // Already reported by resolver
                    NyType::I32
                }
            }
            Expr::If {
                condition,
                then_branch,
                else_branch,
                span,
            } => {
                let cond_ty = self.check_expr(condition);
                if cond_ty != NyType::Bool {
                    self.errors.push(CompileError::type_error(
                        format!("if condition must be bool, found '{}'", cond_ty),
                        condition.span(),
                    ));
                }

                let then_ty = self.check_expr(then_branch);

                if let Some(eb) = else_branch {
                    let else_ty = self.check_expr(eb);
                    if then_ty != else_ty {
                        self.errors.push(CompileError::type_error(
                            format!(
                                "if/else branches have incompatible types: '{}' and '{}'",
                                then_ty, else_ty
                            ),
                            *span,
                        ));
                    }
                    then_ty
                } else {
                    NyType::Unit
                }
            }
            Expr::Block {
                stmts, tail_expr, ..
            } => {
                self.push_scope();
                for stmt in stmts {
                    self.check_stmt(stmt);
                }
                let ty = if let Some(expr) = tail_expr {
                    self.check_expr(expr)
                } else {
                    NyType::Unit
                };
                self.pop_scope();
                ty
            }
        }
    }

    fn check_stmt(&mut self, stmt: &Stmt) {
        match stmt {
            Stmt::VarDecl {
                name,
                ty,
                init,
                span: _,
                ..
            } => {
                let init_ty = self.check_expr(init);
                if let Some(annotation) = ty {
                    if let Some(declared_ty) = NyType::from_name(&annotation.name) {
                        if init_ty != declared_ty {
                            self.errors.push(CompileError::type_error(
                                format!("expected '{}', found '{}'", declared_ty, init_ty),
                                init.span(),
                            ));
                        }
                        self.declare(name, declared_ty);
                    }
                } else {
                    self.declare(name, init_ty);
                }
            }
            Stmt::ConstDecl {
                name,
                ty,
                value,
                span: _,
            } => {
                let val_ty = self.check_expr(value);
                if let Some(annotation) = ty {
                    if let Some(declared_ty) = NyType::from_name(&annotation.name) {
                        if val_ty != declared_ty {
                            self.errors.push(CompileError::type_error(
                                format!("expected '{}', found '{}'", declared_ty, val_ty),
                                value.span(),
                            ));
                        }
                        self.declare(name, declared_ty);
                    }
                } else {
                    self.declare(name, val_ty);
                }
            }
            Stmt::Assign {
                target,
                value,
                span: _,
            } => {
                let val_ty = self.check_expr(value);
                if let Some(expected_ty) = self.lookup(target) {
                    if val_ty != expected_ty {
                        self.errors.push(CompileError::type_error(
                            format!(
                                "cannot assign '{}' to variable '{}' of type '{}'",
                                val_ty, target, expected_ty
                            ),
                            value.span(),
                        ));
                    }
                }
                // Immutability check is done in resolver
            }
            Stmt::ExprStmt { expr, .. } => {
                self.check_expr(expr);
            }
            Stmt::Return { value, span } => {
                let ret_ty = if let Some(v) = value {
                    self.check_expr(v)
                } else {
                    NyType::Unit
                };

                if ret_ty != self.current_return_type {
                    self.errors.push(CompileError::type_error(
                        format!(
                            "return type mismatch: expected '{}', found '{}'",
                            self.current_return_type, ret_ty
                        ),
                        *span,
                    ));
                }
            }
            Stmt::While {
                condition,
                body,
                span: _,
            } => {
                let cond_ty = self.check_expr(condition);
                if cond_ty != NyType::Bool {
                    self.errors.push(CompileError::type_error(
                        format!("while condition must be bool, found '{}'", cond_ty),
                        condition.span(),
                    ));
                }
                self.check_expr(body);
            }
        }
    }
}
