use std::collections::HashMap;

use crate::common::{CompileError, NyType, Span};
use crate::parser::ast::*;

/// Built-in function names — sourced from the central builtin registry.
const BUILTIN_FUNCTIONS: &[&str] = crate::codegen::builtins::BUILTIN_NAMES;

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
    enums: HashMap<String, Vec<(String, Vec<NyType>)>>,
    pub type_aliases: HashMap<String, NyType>,
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
            enums: HashMap::new(),
            type_aliases: HashMap::new(),
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

    /// Find a similar name in scope for "did you mean?" suggestions.
    fn find_similar_name(&self, name: &str) -> Option<String> {
        let mut best: Option<(String, usize)> = None;
        for scope in &self.scopes {
            for key in scope.keys() {
                let dist = crate::common::edit_distance(name, key);
                if dist <= 2 && dist < name.len() && best.as_ref().map_or(true, |(_, d)| dist < *d)
                {
                    best = Some((key.clone(), dist));
                }
            }
        }
        for key in self.functions.keys() {
            let dist = crate::common::edit_distance(name, key);
            if dist <= 2 && dist < name.len() && best.as_ref().map_or(true, |(_, d)| dist < *d) {
                best = Some((key.clone(), dist));
            }
        }
        best.map(|(name, _)| name)
    }

    /// Resolve a TypeAnnotation into a NyType, reporting errors for unknown types.
    fn resolve_type_annotation(&mut self, annotation: &TypeAnnotation) -> Option<NyType> {
        match annotation {
            TypeAnnotation::Named { name, span } => {
                // Try primitive types first
                if let Some(ty) = NyType::from_name(name) {
                    return Some(ty);
                }
                // Check Vec<StructName> pattern
                if let Some(inner) = name.strip_prefix("Vec<").and_then(|s| s.strip_suffix('>')) {
                    if let Some(fields) = self.structs.get(inner) {
                        return Some(NyType::Vec(Box::new(NyType::Struct {
                            name: inner.to_string(),
                            fields: fields.clone(),
                        })));
                    }
                    if let Some(variants) = self.enums.get(inner) {
                        return Some(NyType::Vec(Box::new(NyType::Enum {
                            name: inner.to_string(),
                            variants: variants.clone(),
                        })));
                    }
                }
                // Check HashMap<K,V>
                if let Some(inner) = name
                    .strip_prefix("HashMap<")
                    .and_then(|s| s.strip_suffix('>'))
                {
                    if let Some(comma) = inner.find(',') {
                        let k_str = inner[..comma].trim();
                        let v_str = inner[comma + 1..].trim();
                        let k_ty = NyType::from_name(k_str).unwrap_or(NyType::Str);
                        let v_ty = NyType::from_name(v_str).unwrap_or(NyType::I32);
                        return Some(NyType::HashMap(Box::new(k_ty), Box::new(v_ty)));
                    }
                }
                // Try registered struct types
                if let Some(fields) = self.structs.get(name) {
                    return Some(NyType::Struct {
                        name: name.clone(),
                        fields: fields.clone(),
                    });
                }
                // Try registered enum types
                if let Some(variants) = self.enums.get(name) {
                    return Some(NyType::Enum {
                        name: name.clone(),
                        variants: variants.clone(),
                    });
                }
                // Try type aliases
                if let Some(ty) = self.type_aliases.get(name) {
                    return Some(ty.clone());
                }

                self.errors.push(CompileError::type_error(
                    format!("unknown type '{}'", name),
                    *span,
                ));
                None
            }
            TypeAnnotation::Pointer { inner, span: _ } => self
                .resolve_type_annotation(inner)
                .map(|t| NyType::Pointer(Box::new(t))),
            TypeAnnotation::Optional { inner, span: _ } => self
                .resolve_type_annotation(inner)
                .map(|t| NyType::Optional(Box::new(t))),
            TypeAnnotation::Tuple { types, span: _ } => {
                let mut resolved_types = Vec::new();
                for ty_ann in types {
                    if let Some(resolved) = self.resolve_type_annotation(ty_ann) {
                        resolved_types.push(resolved);
                    } else {
                        return None;
                    }
                }
                Some(NyType::Tuple(resolved_types))
            }
            TypeAnnotation::DynTrait { trait_name, span: _ } => {
                Some(NyType::DynTrait(trait_name.clone()))
            }
            TypeAnnotation::Function {
                params,
                ret,
                span: _,
            } => {
                let mut resolved_params = Vec::new();
                for p in params {
                    if let Some(resolved) = self.resolve_type_annotation(p) {
                        resolved_params.push(resolved);
                    } else {
                        return None;
                    }
                }
                let resolved_ret = if let Some(r) = ret {
                    self.resolve_type_annotation(r)?
                } else {
                    NyType::Unit
                };
                Some(NyType::Function(resolved_params, Box::new(resolved_ret)))
            }
            TypeAnnotation::Array { inner, size, span: _ } => {
                let inner_ty = self.resolve_type_annotation(inner)?;
                Some(NyType::Array(Box::new(inner_ty), *size))
            }
            TypeAnnotation::Slice { inner, span: _ } => {
                let inner_ty = self.resolve_type_annotation(inner)?;
                Some(NyType::Slice(Box::new(inner_ty)))
            }
        }
    }

    pub fn resolve(program: &Program) -> Result<ResolvedInfo, Vec<CompileError>> {
        let mut resolver = Resolver::new();
        resolver.collect_declarations(program);
        resolver.resolve_program(program);

        if resolver.errors.is_empty() {
            Ok(ResolvedInfo {
                functions: resolver.functions,
                structs: resolver.structs,
                enums: resolver.enums,
                type_aliases: resolver.type_aliases,
            })
        } else {
            Err(resolver.errors)
        }
    }

    fn collect_declarations(&mut self, program: &Program) {
        // Collect type aliases first so structs/functions can use them
        for item in &program.items {
            if let Item::TypeAlias { name, ty, span } = item {
                if let Some(resolved_ty) = self.resolve_type_annotation(ty) {
                    self.type_aliases.insert(name.clone(), resolved_ty);
                } else {
                    self.errors.push(CompileError::type_error(
                        format!("invalid type alias '{}'", name),
                        *span,
                    ));
                }
            }
        }

        for item in &program.items {
            match item {
                Item::Struct {
                    name, fields, span, ..
                } => {
                    let mut resolved_fields = Vec::new();
                    for field in fields {
                        if let Some(ty) = self.resolve_type_annotation(&field.ty) {
                            resolved_fields.push((field.name.clone(), ty));
                        }
                    }
                    if self.structs.contains_key(name) {
                        self.errors.push(CompileError::name_error(
                            format!("struct '{}' is already defined", name),
                            *span,
                        ));
                    }
                    self.structs.insert(name.clone(), resolved_fields);
                }
                Item::Enum {
                    name,
                    variants,
                    span,
                    ..
                } => {
                    let mut resolved_variants = Vec::new();
                    for variant in variants {
                        let mut payload_types = Vec::new();
                        if let Some(payload) = &variant.payload {
                            for ty_ann in payload {
                                if let Some(ty) = self.resolve_type_annotation(ty_ann) {
                                    payload_types.push(ty);
                                }
                            }
                        }
                        resolved_variants.push((variant.name.clone(), payload_types));
                    }
                    if self.enums.contains_key(name) {
                        self.errors.push(CompileError::name_error(
                            format!("enum '{}' is already defined", name),
                            *span,
                        ));
                    }
                    self.enums.insert(name.clone(), resolved_variants);
                }
                Item::Function {
                    name,
                    params,
                    ret,
                    span,
                    ..
                } => {
                    let mut param_types = Vec::new();
                    for p in params {
                        if let Some(ty) = self.resolve_type_annotation(&p.ty) {
                            param_types.push(ty);
                        } else {
                            param_types.push(NyType::Unit); // placeholder
                        }
                    }
                    let ret_type = if let Some(ret_ann) = ret {
                        self.resolve_type_annotation(ret_ann).unwrap_or(NyType::Unit)
                    } else {
                        NyType::Unit
                    };
                    if self.functions.contains_key(name) {
                        self.errors.push(CompileError::name_error(
                            format!("function '{}' is already defined", name),
                            *span,
                        ));
                    }
                    self.functions
                        .insert(name.clone(), (param_types, ret_type, *span));
                }
                _ => {}
            }
        }
    }

    fn resolve_program(&mut self, program: &Program) {
        for item in &program.items {
            match item {
                Item::Function { params, body, .. } => {
                    self.push_scope();
                    for param in params {
                        let ty = self
                            .resolve_type_annotation(&param.ty)
                            .unwrap_or(NyType::Unit);
                        self.declare(
                            &param.name,
                            Symbol {
                                name: param.name.clone(),
                                ty,
                                mutability: param.mutability,
                                span: param.span,
                            },
                        );
                    }
                    if let Some(b) = body {
                        self.resolve_block(b);
                    }
                    self.pop_scope();
                }
                Item::Trait { methods, .. } => {
                    for method in methods {
                        if let Some(body) = &method.body {
                            self.push_scope();
                            for param in &method.params {
                                let ty = self
                                    .resolve_type_annotation(&param.ty)
                                    .unwrap_or(NyType::Unit);
                                self.declare(
                                    &param.name,
                                    Symbol {
                                        name: param.name.clone(),
                                        ty,
                                        mutability: param.mutability,
                                        span: param.span,
                                    },
                                );
                            }
                            self.resolve_block(body);
                            self.pop_scope();
                        }
                    }
                }
                Item::Impl { methods, .. } => {
                    for method in methods {
                        if let Some(body) = &method.body {
                            self.push_scope();
                            for param in &method.params {
                                let ty = self
                                    .resolve_type_annotation(&param.ty)
                                    .unwrap_or(NyType::Unit);
                                self.declare(
                                    &param.name,
                                    Symbol {
                                        name: param.name.clone(),
                                        ty,
                                        mutability: param.mutability,
                                        span: param.span,
                                    },
                                );
                            }
                            self.resolve_block(body);
                            self.pop_scope();
                        }
                    }
                }
                Item::Test { body, .. } => {
                    self.push_scope();
                    self.resolve_block(body);
                    self.pop_scope();
                }
                _ => {}
            }
        }
    }

    fn resolve_block(&mut self, block: &Block) {
        self.push_scope();
        for stmt in &block.stmts {
            self.resolve_stmt(stmt);
        }
        if let Some(expr) = &block.expr {
            self.resolve_expr(expr);
        }
        self.pop_scope();
    }

    fn resolve_stmt(&mut self, stmt: &Stmt) {
        match stmt {
            Stmt::Let {
                name,
                ty,
                init,
                mutability,
                span,
            } => {
                if let Some(expr) = init {
                    self.resolve_expr(expr);
                }
                let resolved_ty = if let Some(t) = ty {
                    self.resolve_type_annotation(t).unwrap_or(NyType::Unit)
                } else {
                    NyType::Unit // Will be inferred later
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
            Stmt::Expr(expr) => self.resolve_expr(expr),
            Stmt::Return(expr, _) => {
                if let Some(e) = expr {
                    self.resolve_expr(e);
                }
            }
            Stmt::Break(_) | Stmt::Continue(_) => {
                if self.loop_depth == 0 {
                    let span = match stmt {
                        Stmt::Break(s) => *s,
                        Stmt::Continue(s) => *s,
                        _ => unreachable!(),
                    };
                    self.errors.push(CompileError::syntax(
                        "cannot use loop control outside of a loop",
                        span,
                    ));
                }
            }
            Stmt::While { cond, body, .. } => {
                self.resolve_expr(cond);
                self.loop_depth += 1;
                self.resolve_block(body);
                self.loop_depth -= 1;
            }
            Stmt::For {
                var,
                iterable,
                body,
                ..
            } => {
                self.resolve_expr(iterable);
                self.push_scope();
                self.declare(
                    var,
                    Symbol {
                        name: var.clone(),
                        ty: NyType::Unit, // Will be inferred
                        mutability: Mutability::Immutable,
                        span: iterable.span(),
                    },
                );
                self.loop_depth += 1;
                self.resolve_block(body);
                self.loop_depth -= 1;
                self.pop_scope();
            }
            Stmt::Loop { body, .. } => {
                self.loop_depth += 1;
                self.resolve_block(body);
                self.loop_depth -= 1;
            }
            Stmt::Defer { expr, .. } => {
                self.resolve_expr(expr);
            }
        }
    }

    fn resolve_expr(&mut self, expr: &Expr) {
        match expr {
            Expr::Ident { name, span } => {
                if self.resolve_name(name).is_none()
                    && !self.functions.contains_key(name)
                    && !Self::is_builtin(name)
                {
                    let mut err = CompileError::name_error(
                        format!("cannot find value '{}' in this scope", name),
                        *span,
                    );
                    if let Some(similar) = self.find_similar_name(name) {
                        err = err.with_note(format!("did you mean '{}'?", similar));
                    }
                    self.errors.push(err);
                }
            }
            Expr::Assign { left, right, span } => {
                self.resolve_expr(left);
                self.resolve_expr(right);

                // Check mutability for simple assignments
                if let Expr::Ident { name, .. } = &**left {
                    if let Some(sym) = self.resolve_name(name) {
                        if sym.mutability == Mutability::Immutable {
                            self.errors.push(CompileError::immutability(
                                format!("cannot assign twice to immutable variable '{}'", name),
                                *span,
                            ));
                        }
                    }
                }
            }
            Expr::Binary { left, right, .. } => {
                self.resolve_expr(left);
                self.resolve_expr(right);
            }
            Expr::Unary { right, .. } => {
                self.resolve_expr(right);
            }
            Expr::Call { callee, args, span } => {
                self.resolve_expr(callee);
                for arg in args {
                    self.resolve_expr(arg);
                }
                
                // Add more context for undefined function calls
                if let Expr::Ident { name, .. } = &**callee {
                    if self.resolve_name(name).is_none()
                        && !self.functions.contains_key(name)
                        && !Self::is_builtin(name)
                    {
                        // Note: The error is already reported by the Ident resolution,
                        // but we could add a specific function-call hint here if needed.
                    }
                }
            }
            Expr::MethodCall { callee, args, .. } => {
                self.resolve_expr(callee);
                for arg in args {
                    self.resolve_expr(arg);
                }
            }
            Expr::If {
                cond,
                then_branch,
                else_branch,
                ..
            } => {
                self.resolve_expr(cond);
                self.resolve_block(then_branch);
                if let Some(e) = else_branch {
                    self.resolve_block(e);
                }
            }
            Expr::Match { expr, arms, .. } => {
                self.resolve_expr(expr);
                for arm in arms {
                    self.push_scope();
                    self.resolve_pattern(&arm.pattern);
                    if let Some(guard) = &arm.guard {
                        self.resolve_expr(guard);
                    }
                    self.resolve_expr(&arm.body);
                    self.pop_scope();
                }
            }
            Expr::StructInit { name, fields, span } => {
                if !self.structs.contains_key(name) {
                    let mut err = CompileError::type_error(
                        format!("cannot construct unknown struct '{}'", name),
                        *span,
                    );
                    
                    // Suggest similar struct names
                    let mut best: Option<(String, usize)> = None;
                    for key in self.structs.keys() {
                        let dist = crate::common::edit_distance(name, key);
                        if dist <= 2 && dist < name.len() && best.as_ref().map_or(true, |(_, d)| dist < *d) {
                            best = Some((key.clone(), dist));
                        }
                    }
                    if let Some((similar, _)) = best {
                        err = err.with_note(format!("did you mean struct '{}'?", similar));
                    }
                    
                    self.errors.push(err);
                }
                for field in fields {
                    self.resolve_expr(&field.value);
                }
            }
            Expr::EnumInit {
                enum_name, payload, span, ..
            } => {
                if !self.enums.contains_key(enum_name) {
                    self.errors.push(CompileError::type_error(
                        format!("cannot construct unknown enum '{}'", enum_name),
                        *span,
                    ));
                }
                if let Some(p) = payload {
                    for expr in p {
                        self.resolve_expr(expr);
                    }
                }
            }
            Expr::FieldAccess { expr, .. } => {
                self.resolve_expr(expr);
            }
            Expr::Index { array, index, .. } => {
                self.resolve_expr(array);
                self.resolve_expr(index);
            }
            Expr::Array { elements, .. } => {
                for el in elements {
                    self.resolve_expr(el);
                }
            }
            Expr::Tuple { elements, .. } => {
                for el in elements {
                    self.resolve_expr(el);
                }
            }
            Expr::Closure { params, body, .. } => {
                self.push_scope();
                for param in params {
                    let ty = self
                        .resolve_type_annotation(&param.ty)
                        .unwrap_or(NyType::Unit);
                    self.declare(
                        &param.name,
                        Symbol {
                            name: param.name.clone(),
                            ty,
                            mutability: Mutability::Immutable,
                            span: param.span,
                        },
                    );
                }
                self.resolve_expr(body);
                self.pop_scope();
            }
            Expr::Spawn { closure, .. } => {
                self.resolve_expr(closure);
            }
            Expr::Await { expr, .. } => {
                self.resolve_expr(expr);
            }
            Expr::ChannelSend { channel, value, .. } => {
                self.resolve_expr(channel);
                self.resolve_expr(value);
            }
            Expr::ChannelReceive { channel, .. } => {
                self.resolve_expr(channel);
            }
            Expr::Select { arms, .. } => {
                for arm in arms {
                    self.push_scope();
                    match &arm.pattern {
                        SelectPattern::Receive(name, expr) => {
                            self.resolve_expr(expr);
                            self.declare(
                                name,
                                Symbol {
                                    name: name.clone(),
                                    ty: NyType::Unit, // Will be inferred
                                    mutability: Mutability::Immutable,
                                    span: expr.span(),
                                },
                            );
                        }
                        SelectPattern::Send(channel, value) => {
                            self.resolve_expr(channel);
                            self.resolve_expr(value);
                        }
                    }
                    self.resolve_expr(&arm.body);
                    self.pop_scope();
                }
            }
            Expr::Cast { expr, ty, span } => {
                self.resolve_expr(expr);
                if self.resolve_type_annotation(ty).is_none() {
                    self.errors.push(CompileError::type_error(
                        format!("invalid cast target type"),
                        *span,
                    ));
                }
            }
            Expr::Literal(_) | Expr::FString { .. } => {}
        }
    }

    fn resolve_pattern(&mut self, pat: &Pattern) {
        match pat {
            Pattern::Ident(name, span) => {
                if name != "_" {
                    self.declare(
                        name,
                        Symbol {
                            name: name.clone(),
                            ty: NyType::Unit, // Infer later
                            mutability: Mutability::Immutable,
                            span: *span,
                        },
                    );
                }
            }
            Pattern::Tuple(patterns) => {
                for p in patterns {
                    self.resolve_pattern(p);
                }
            }
            Pattern::EnumVariant { payload, .. } => {
                if let Some(p) = payload {
                    for pat in p {
                        self.resolve_pattern(pat);
                    }
                }
            }
            Pattern::Literal(_) | Pattern::Wildcard => {}
        }
    }
}

pub struct ResolvedInfo {
    pub functions: HashMap<String, (Vec<NyType>, NyType, Span)>,
    pub structs: HashMap<String, Vec<(String, NyType)>>,
    pub enums: HashMap<String, Vec<(String, Vec<NyType>)>>,
    pub type_aliases: HashMap<String, NyType>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::token::TokenKind;

    fn dummy_span() -> Span {
        Span { line: 1, column: 1, length: 1 }
    }

    #[test]
    fn test_resolve_unknown_struct_with_suggestion() {
        let mut resolver = Resolver::new();
        resolver.structs.insert("MyStruct".to_string(), vec![]);
        
        let expr = Expr::StructInit {
            name: "MyStrcut".to_string(),
            fields: vec![],
            span: dummy_span(),
        };
        
        resolver.resolve_expr(&expr);
        
        assert_eq!(resolver.errors.len(), 1);
        let err = &resolver.errors[0];
        assert_eq!(err.message, "cannot construct unknown struct 'MyStrcut'");
        assert_eq!(err.notes.len(), 1);
        assert_eq!(err.notes[0], "did you mean struct 'MyStruct'?");
    }
    
    #[test]
    fn test_resolve_unknown_variable_with_suggestion() {
        let mut resolver = Resolver::new();
        resolver.push_scope();
        resolver.declare("my_variable", Symbol {
            name: "my_variable".to_string(),
            ty: NyType::I32,
            mutability: Mutability::Immutable,
            span: dummy_span(),
        });
        
        let expr = Expr::Ident {
            name: "my_varibale".to_string(),
            span: dummy_span(),
        };
        
        resolver.resolve_expr(&expr);
        
        assert_eq!(resolver.errors.len(), 1);
        let err = &resolver.errors[0];
        assert_eq!(err.message, "cannot find value 'my_varibale' in this scope");
        assert_eq!(err.notes.len(), 1);
        assert_eq!(err.notes[0], "did you mean 'my_variable'?");
    }

    #[test]
    fn test_resolve_immutable_assignment_error() {
        let mut resolver = Resolver::new();
        resolver.push_scope();
        resolver.declare("x", Symbol {
            name: "x".to_string(),
            ty: NyType::I32,
            mutability: Mutability::Immutable,
            span: dummy_span(),
        });

        let expr = Expr::Assign {
            left: Box::new(Expr::Ident { name: "x".to_string(), span: dummy_span() }),
            right: Box::new(Expr::Literal(crate::parser::ast::Literal::Int(1))),
            span: dummy_span(),
        };

        resolver.resolve_expr(&expr);

        assert_eq!(resolver.errors.len(), 1);
        let err = &resolver.errors[0];
        assert_eq!(err.message, "cannot assign twice to immutable variable 'x'");
    }
}
