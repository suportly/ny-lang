use std::collections::HashMap;

use crate::common::{CompileError, NyType, Span};
use crate::parser::ast::*;

#[derive(Debug, Clone)]
pub struct Symbol {
    pub name: String,
    pub ty: NyType,
    pub mutability: Mutability,
    pub span: Span,
}

pub struct Resolver {
    scopes: Vec<HashMap<String, Symbol>>,
    functions: HashMap<String, (Vec<NyType>, NyType, Span)>,
    errors: Vec<CompileError>,
}

impl Default for Resolver {
    fn default() -> Self {
        Self::new()
    }
}

impl Resolver {
    pub fn new() -> Self {
        Self {
            scopes: vec![HashMap::new()],
            functions: HashMap::new(),
            errors: Vec::new(),
        }
    }

    fn push_scope(&mut self) {
        self.scopes.push(HashMap::new());
    }

    fn pop_scope(&mut self) {
        self.scopes.pop();
    }

    fn declare(&mut self, name: &str, symbol: Symbol) {
        if let Some(existing) = self.scopes.last().unwrap().get(name) {
            self.errors.push(
                CompileError::name_error(
                    format!("duplicate declaration of '{}'", name),
                    symbol.span,
                )
                .with_secondary(existing.span, "previously declared here".to_string()),
            );
            return;
        }
        self.scopes
            .last_mut()
            .unwrap()
            .insert(name.to_string(), symbol);
    }

    fn resolve_name(&self, name: &str) -> Option<&Symbol> {
        for scope in self.scopes.iter().rev() {
            if let Some(sym) = scope.get(name) {
                return Some(sym);
            }
        }
        None
    }

    pub fn resolve_type(name: &str) -> Option<NyType> {
        NyType::from_name(name)
    }

    pub fn resolve(program: &Program) -> Result<ResolvedInfo, Vec<CompileError>> {
        let mut resolver = Resolver::new();

        // First pass: register all function signatures (forward references)
        for item in &program.items {
            match item {
                Item::FunctionDef {
                    name,
                    params,
                    return_type,
                    span,
                    ..
                } => {
                    let param_types: Vec<NyType> = params
                        .iter()
                        .filter_map(|p| Self::resolve_type(&p.ty.name))
                        .collect();

                    if param_types.len() != params.len() {
                        for p in params {
                            if Self::resolve_type(&p.ty.name).is_none() {
                                resolver.errors.push(CompileError::type_error(
                                    format!("unknown type '{}'", p.ty.name),
                                    p.ty.span,
                                ));
                            }
                        }
                    }

                    let ret_type = Self::resolve_type(&return_type.name).unwrap_or_else(|| {
                        resolver.errors.push(CompileError::type_error(
                            format!("unknown type '{}'", return_type.name),
                            return_type.span,
                        ));
                        NyType::Unit
                    });

                    resolver
                        .functions
                        .insert(name.clone(), (param_types, ret_type, *span));
                }
            }
        }

        // Second pass: resolve bodies
        for item in &program.items {
            match item {
                Item::FunctionDef {
                    name: _,
                    params,
                    body,
                    ..
                } => {
                    resolver.push_scope();
                    for p in params {
                        if let Some(ty) = Self::resolve_type(&p.ty.name) {
                            resolver.declare(
                                &p.name,
                                Symbol {
                                    name: p.name.clone(),
                                    ty,
                                    mutability: Mutability::Immutable,
                                    span: p.span,
                                },
                            );
                        }
                    }
                    resolver.resolve_expr(body);
                    resolver.pop_scope();
                }
            }
        }

        if resolver.errors.is_empty() {
            Ok(ResolvedInfo {
                functions: resolver.functions,
            })
        } else {
            Err(resolver.errors)
        }
    }

    fn resolve_expr(&mut self, expr: &Expr) {
        match expr {
            Expr::Literal { .. } => {}
            Expr::Ident { name, span } => {
                if self.resolve_name(name).is_none() && !self.functions.contains_key(name) {
                    self.errors.push(CompileError::name_error(
                        format!("undeclared variable '{}'", name),
                        *span,
                    ));
                }
            }
            Expr::BinOp { lhs, rhs, .. } => {
                self.resolve_expr(lhs);
                self.resolve_expr(rhs);
            }
            Expr::UnaryOp { operand, .. } => {
                self.resolve_expr(operand);
            }
            Expr::Call { callee, args, span } => {
                if !self.functions.contains_key(callee) {
                    self.errors.push(CompileError::name_error(
                        format!("undeclared function '{}'", callee),
                        *span,
                    ));
                }
                for arg in args {
                    self.resolve_expr(arg);
                }
            }
            Expr::If {
                condition,
                then_branch,
                else_branch,
                ..
            } => {
                self.resolve_expr(condition);
                self.resolve_expr(then_branch);
                if let Some(eb) = else_branch {
                    self.resolve_expr(eb);
                }
            }
            Expr::Block {
                stmts, tail_expr, ..
            } => {
                self.push_scope();
                for stmt in stmts {
                    self.resolve_stmt(stmt);
                }
                if let Some(expr) = tail_expr {
                    self.resolve_expr(expr);
                }
                self.pop_scope();
            }
        }
    }

    fn resolve_stmt(&mut self, stmt: &Stmt) {
        match stmt {
            Stmt::VarDecl {
                name,
                mutability,
                ty,
                init,
                span,
            } => {
                self.resolve_expr(init);
                let resolved_ty = ty
                    .as_ref()
                    .and_then(|t| Self::resolve_type(&t.name))
                    .unwrap_or(NyType::I32);

                if let Some(t) = ty {
                    if Self::resolve_type(&t.name).is_none() {
                        self.errors.push(CompileError::type_error(
                            format!("unknown type '{}'", t.name),
                            t.span,
                        ));
                    }
                }

                self.declare(
                    name,
                    Symbol {
                        name: name.clone(),
                        ty: resolved_ty,
                        mutability: *mutability,
                        span: *span,
                    },
                );
            }
            Stmt::ConstDecl {
                name,
                ty,
                value,
                span,
            } => {
                self.resolve_expr(value);
                let resolved_ty = ty
                    .as_ref()
                    .and_then(|t| Self::resolve_type(&t.name))
                    .unwrap_or(NyType::I32);

                if let Some(t) = ty {
                    if Self::resolve_type(&t.name).is_none() {
                        self.errors.push(CompileError::type_error(
                            format!("unknown type '{}'", t.name),
                            t.span,
                        ));
                    }
                }

                self.declare(
                    name,
                    Symbol {
                        name: name.clone(),
                        ty: resolved_ty,
                        mutability: Mutability::Immutable,
                        span: *span,
                    },
                );
            }
            Stmt::Assign {
                target,
                value,
                span,
            } => {
                self.resolve_expr(value);
                match self.resolve_name(target) {
                    None => {
                        self.errors.push(CompileError::name_error(
                            format!("undeclared variable '{}'", target),
                            *span,
                        ));
                    }
                    Some(sym) if sym.mutability == Mutability::Immutable => {
                        let decl_span = sym.span;
                        self.errors.push(
                            CompileError::immutability(
                                format!("cannot assign to immutable variable '{}'", target),
                                *span,
                            )
                            .with_secondary(decl_span, "declared as immutable here".to_string()),
                        );
                    }
                    Some(_) => {}
                }
            }
            Stmt::ExprStmt { expr, .. } => {
                self.resolve_expr(expr);
            }
            Stmt::Return { value, .. } => {
                if let Some(v) = value {
                    self.resolve_expr(v);
                }
            }
            Stmt::While {
                condition, body, ..
            } => {
                self.resolve_expr(condition);
                self.resolve_expr(body);
            }
        }
    }
}

pub struct ResolvedInfo {
    pub functions: HashMap<String, (Vec<NyType>, NyType, Span)>,
}
