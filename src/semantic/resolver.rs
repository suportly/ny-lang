use std::collections::HashMap;

use crate::common::{CompileError, NyType, Span};
use crate::parser::ast::*;

/// Built-in function names that should not trigger "undeclared function" errors.
const BUILTIN_FUNCTIONS: &[&str] = &["print", "println"];

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
    structs: HashMap<String, Vec<(String, NyType)>>,
    errors: Vec<CompileError>,
    loop_depth: usize,
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
            structs: HashMap::new(),
            errors: Vec::new(),
            loop_depth: 0,
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

    fn is_builtin(name: &str) -> bool {
        BUILTIN_FUNCTIONS.contains(&name)
    }

    /// Resolve a TypeAnnotation into a NyType, reporting errors for unknown types.
    fn resolve_type_annotation(&mut self, annotation: &TypeAnnotation) -> Option<NyType> {
        match annotation {
            TypeAnnotation::Named { name, span } => {
                // Try primitive types first
                if let Some(ty) = NyType::from_name(name) {
                    return Some(ty);
                }
                // Try registered struct types
                if let Some(fields) = self.structs.get(name) {
                    return Some(NyType::Struct {
                        name: name.clone(),
                        fields: fields.clone(),
                    });
                }
                // "unit" / "()" as a special case for return types
                if name == "()" {
                    return Some(NyType::Unit);
                }
                self.errors.push(CompileError::type_error(
                    format!("unknown type '{}'", name),
                    *span,
                ));
                None
            }
            TypeAnnotation::Array {
                elem,
                size,
                span: _,
            } => {
                let elem_ty = self.resolve_type_annotation(elem)?;
                Some(NyType::Array {
                    elem: Box::new(elem_ty),
                    size: *size,
                })
            }
            TypeAnnotation::Pointer { inner, span: _ } => {
                let inner_ty = self.resolve_type_annotation(inner)?;
                Some(NyType::Pointer(Box::new(inner_ty)))
            }
        }
    }

    pub fn resolve(program: &Program) -> Result<ResolvedInfo, Vec<CompileError>> {
        let mut resolver = Resolver::new();

        // ---- Pass 1: Register all struct definitions ----
        for item in &program.items {
            if let Item::StructDef { name, fields, span } = item {
                if resolver.structs.contains_key(name) {
                    resolver.errors.push(CompileError::name_error(
                        format!("duplicate struct definition '{}'", name),
                        *span,
                    ));
                    continue;
                }
                // Collect field types — we resolve them against already-registered structs
                // and primitives. Forward references between structs are supported as long as
                // the referenced struct appears earlier in the source (or is the same struct
                // for self-referential pointers).
                let mut resolved_fields: Vec<(String, NyType)> = Vec::new();
                let mut had_error = false;
                for (field_name, field_ty_ann) in fields {
                    if let Some(field_ty) = resolver.resolve_type_annotation(field_ty_ann) {
                        resolved_fields.push((field_name.clone(), field_ty));
                    } else {
                        had_error = true;
                    }
                }
                if !had_error {
                    resolver.structs.insert(name.clone(), resolved_fields);
                }
            }
        }

        // ---- Pass 2: Register all function signatures (forward references) ----
        for item in &program.items {
            if let Item::FunctionDef {
                name,
                params,
                return_type,
                span,
                ..
            } = item
            {
                let mut param_types: Vec<NyType> = Vec::new();
                let mut all_ok = true;

                for p in params {
                    if let Some(ty) = resolver.resolve_type_annotation(&p.ty) {
                        param_types.push(ty);
                    } else {
                        all_ok = false;
                    }
                }

                if !all_ok {
                    // Errors already pushed inside resolve_type_annotation
                    // Still register the function with whatever we have so far to
                    // avoid cascading "undeclared function" errors
                    let ret_type = resolver
                        .resolve_type_annotation(return_type)
                        .unwrap_or(NyType::Unit);
                    resolver
                        .functions
                        .insert(name.clone(), (param_types, ret_type, *span));
                    continue;
                }

                let ret_type = resolver
                    .resolve_type_annotation(return_type)
                    .unwrap_or(NyType::Unit);

                resolver
                    .functions
                    .insert(name.clone(), (param_types, ret_type, *span));
            }
        }

        // ---- Pass 3: Resolve all function bodies ----
        for item in &program.items {
            if let Item::FunctionDef { params, body, .. } = item {
                resolver.push_scope();
                for p in params {
                    if let Some(ty) = resolver.resolve_type_annotation(&p.ty) {
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

        if resolver.errors.is_empty() {
            Ok(ResolvedInfo {
                functions: resolver.functions,
                structs: resolver.structs,
            })
        } else {
            Err(resolver.errors)
        }
    }

    fn resolve_expr(&mut self, expr: &Expr) {
        match expr {
            Expr::Literal { value, .. } => {
                // LitValue::Int, Float, Bool, Str — all valid, nothing to resolve
                match value {
                    LitValue::Int(_)
                    | LitValue::Float(_)
                    | LitValue::Bool(_)
                    | LitValue::Str(_) => {}
                }
            }
            Expr::Ident { name, span } => {
                if self.resolve_name(name).is_none()
                    && !self.functions.contains_key(name)
                    && !Self::is_builtin(name)
                    && !self.structs.contains_key(name)
                {
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
                if !self.functions.contains_key(callee) && !Self::is_builtin(callee) {
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
            // ---- Phase 2: Array expressions ----
            Expr::ArrayLit { elements, .. } => {
                for elem in elements {
                    self.resolve_expr(elem);
                }
            }
            Expr::Index { object, index, .. } => {
                self.resolve_expr(object);
                self.resolve_expr(index);
            }
            // ---- Phase 2: Struct expressions ----
            Expr::StructInit { name, fields, span } => {
                // Verify the struct type exists
                if !self.structs.contains_key(name) {
                    self.errors.push(CompileError::name_error(
                        format!("undeclared struct type '{}'", name),
                        *span,
                    ));
                }
                // Resolve all field value expressions
                for (_field_name, field_expr) in fields {
                    self.resolve_expr(field_expr);
                }
            }
            Expr::FieldAccess { object, .. } => {
                self.resolve_expr(object);
                // Field name validation is deferred to the type checker, which
                // knows the concrete type of `object`.
            }
            Expr::MethodCall { object, args, .. } => {
                self.resolve_expr(object);
                for arg in args {
                    self.resolve_expr(arg);
                }
                // Method resolution is deferred to the type checker.
            }
            // ---- Phase 2: Pointer expressions ----
            Expr::AddrOf { operand, .. } => {
                self.resolve_expr(operand);
            }
            Expr::Deref { operand, .. } => {
                self.resolve_expr(operand);
            }
            Expr::Cast { expr, .. } => {
                self.resolve_expr(expr);
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

                // Resolve the optional type annotation
                let resolved_ty = if let Some(ann) = ty {
                    self.resolve_type_annotation(ann).unwrap_or(NyType::I32)
                } else {
                    // Type inference: no annotation provided (`:=` or `:~=` syntax).
                    // The actual inferred type will be determined by the type checker;
                    // here we use I32 as a placeholder.
                    NyType::I32
                };

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

                let resolved_ty = if let Some(ann) = ty {
                    self.resolve_type_annotation(ann).unwrap_or(NyType::I32)
                } else {
                    NyType::I32
                };

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
                self.resolve_assign_target(target, *span);
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
                self.loop_depth += 1;
                self.resolve_expr(body);
                self.loop_depth -= 1;
            }
            // ---- Phase 2: For-range loops ----
            Stmt::ForRange {
                var,
                start,
                end,
                body,
                span,
                ..
            } => {
                // Resolve start and end expressions in the current scope
                self.resolve_expr(start);
                self.resolve_expr(end);

                // Open a new scope for the loop body and declare the loop variable
                self.push_scope();
                self.declare(
                    var,
                    Symbol {
                        name: var.clone(),
                        ty: NyType::I32, // Default; type checker will refine
                        mutability: Mutability::Immutable,
                        span: *span,
                    },
                );
                self.loop_depth += 1;
                self.resolve_expr(body);
                self.loop_depth -= 1;
                self.pop_scope();
            }
            // ---- Phase 2: Break / Continue ----
            Stmt::Break { span } => {
                if self.loop_depth == 0 {
                    self.errors.push(CompileError::syntax(
                        "'break' used outside of a loop",
                        *span,
                    ));
                }
            }
            Stmt::Continue { span } => {
                if self.loop_depth == 0 {
                    self.errors.push(CompileError::syntax(
                        "'continue' used outside of a loop",
                        *span,
                    ));
                }
            }
        }
    }

    /// Resolve an assignment target, checking that the target variable exists
    /// and is mutable, or recursively resolving sub-expressions for complex
    /// targets (index, field, deref).
    fn resolve_assign_target(&mut self, target: &AssignTarget, span: Span) {
        match target {
            AssignTarget::Var(name) => match self.resolve_name(name) {
                None => {
                    self.errors.push(CompileError::name_error(
                        format!("undeclared variable '{}'", name),
                        span,
                    ));
                }
                Some(sym) if sym.mutability == Mutability::Immutable => {
                    let decl_span = sym.span;
                    self.errors.push(
                        CompileError::immutability(
                            format!("cannot assign to immutable variable '{}'", name),
                            span,
                        )
                        .with_secondary(decl_span, "declared as immutable here".to_string()),
                    );
                }
                Some(_) => {}
            },
            AssignTarget::Index(object_expr, index_expr) => {
                // Resolve both sub-expressions; mutability of the indexed container
                // is checked by the type checker.
                self.resolve_expr(object_expr);
                self.resolve_expr(index_expr);
            }
            AssignTarget::Field(object_expr, _field_name) => {
                // Resolve the object expression; field existence is checked
                // by the type checker.
                self.resolve_expr(object_expr);
            }
            AssignTarget::Deref(operand_expr) => {
                // Resolve the pointer expression being dereferenced.
                self.resolve_expr(operand_expr);
            }
        }
    }
}

pub struct ResolvedInfo {
    pub functions: HashMap<String, (Vec<NyType>, NyType, Span)>,
    pub structs: HashMap<String, Vec<(String, NyType)>>,
}
