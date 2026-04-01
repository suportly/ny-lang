use std::collections::HashMap;

use crate::common::{CompileError, NyType, Span};
use crate::parser::ast::*;
use crate::semantic::resolver::ResolvedInfo;

pub struct TypeChecker {
    scopes: Vec<HashMap<String, NyType>>,
    functions: HashMap<String, (Vec<NyType>, NyType)>,
    /// Struct definitions: struct_name → fields
    structs: HashMap<String, Vec<(String, NyType)>>,
    /// Function parameter names (needed for method call self-parameter detection)
    function_params: HashMap<String, Vec<String>>,
    current_return_type: NyType,
    errors: Vec<CompileError>,
    loop_depth: usize,
}

impl TypeChecker {
    pub fn new(resolved: &ResolvedInfo) -> Self {
        let functions = resolved
            .functions
            .iter()
            .map(|(name, (params, ret, _))| (name.clone(), (params.clone(), ret.clone())))
            .collect();

        let structs = resolved.structs.clone();

        Self {
            scopes: vec![HashMap::new()],
            functions,
            structs,
            function_params: HashMap::new(),
            current_return_type: NyType::Unit,
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

    /// Resolve a TypeAnnotation into a NyType, using struct definitions when needed.
    fn resolve_type_annotation(&self, ann: &TypeAnnotation) -> Option<NyType> {
        match ann {
            TypeAnnotation::Named { name, .. } => {
                // Check primitive types first
                if let Some(ty) = NyType::from_name(name) {
                    return Some(ty);
                }
                // Check struct types
                if let Some(fields) = self.structs.get(name) {
                    return Some(NyType::Struct {
                        name: name.clone(),
                        fields: fields.clone(),
                    });
                }
                // "unit" or "()" convention
                if name == "()" {
                    return Some(NyType::Unit);
                }
                None
            }
            TypeAnnotation::Array { elem, size, .. } => {
                let elem_ty = self.resolve_type_annotation(elem)?;
                Some(NyType::Array {
                    elem: Box::new(elem_ty),
                    size: *size,
                })
            }
            TypeAnnotation::Pointer { inner, .. } => {
                let inner_ty = self.resolve_type_annotation(inner)?;
                Some(NyType::Pointer(Box::new(inner_ty)))
            }
        }
    }

    pub fn check(program: &Program, resolved: &ResolvedInfo) -> Result<(), Vec<CompileError>> {
        let mut checker = TypeChecker::new(resolved);

        // Collect function parameter names from the AST for method-call self detection
        for item in &program.items {
            if let Item::FunctionDef { name, params, .. } = item {
                let param_names: Vec<String> = params.iter().map(|p| p.name.clone()).collect();
                checker.function_params.insert(name.clone(), param_names);
            }
        }

        for item in &program.items {
            checker.check_item(item);
        }

        // Verify main function exists and returns i32
        match resolved.functions.get("main") {
            None => {
                checker.errors.push(CompileError::name_error(
                    "no 'main' function found",
                    Span::empty(0),
                ));
            }
            Some((params, ret, span)) => {
                if !params.is_empty() {
                    checker.errors.push(CompileError::type_error(
                        "'main' function must have no parameters",
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
                params,
                return_type,
                body,
                ..
            } => {
                let ret_ty = self
                    .resolve_type_annotation(return_type)
                    .unwrap_or(NyType::Unit);
                self.current_return_type = ret_ty;

                self.push_scope();
                for p in params {
                    if let Some(ty) = self.resolve_type_annotation(&p.ty) {
                        self.declare(&p.name, ty);
                    }
                }
                self.check_expr(body);
                self.pop_scope();
            }
            Item::StructDef { .. } => {
                // Struct definitions are already registered in ResolvedInfo.
                // No additional type checking needed at definition site.
            }
        }
    }

    fn check_expr(&mut self, expr: &Expr) -> NyType {
        match expr {
            // ── Literals ──────────────────────────────────────────────
            Expr::Literal { value, .. } => match value {
                LitValue::Int(_) => NyType::I32,
                LitValue::Float(_) => NyType::F64,
                LitValue::Bool(_) => NyType::Bool,
                LitValue::Str(_) => NyType::Str,
            },

            // ── Identifier ────────────────────────────────────────────
            Expr::Ident { name, .. } => {
                self.lookup(name).unwrap_or(
                    // Already reported by resolver
                    NyType::I32,
                )
            }

            // ── Binary operations ─────────────────────────────────────
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

            // ── Unary operations ──────────────────────────────────────
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

            // ── Function call ─────────────────────────────────────────
            Expr::Call { callee, args, span } => {
                // Built-in print/println: accept any single argument, return Unit
                if callee == "print" || callee == "println" {
                    for arg in args {
                        self.check_expr(arg);
                    }
                    return NyType::Unit;
                }

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

            // ── If expression ─────────────────────────────────────────
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

            // ── Block ─────────────────────────────────────────────────
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

            // ── Array literal ─────────────────────────────────────────
            Expr::ArrayLit { elements, span } => {
                if elements.is_empty() {
                    // Empty array literal -- cannot infer element type
                    self.errors.push(CompileError::type_error(
                        "cannot infer type of empty array literal",
                        *span,
                    ));
                    return NyType::Array {
                        elem: Box::new(NyType::Unit),
                        size: 0,
                    };
                }

                let first_ty = self.check_expr(&elements[0]);
                for (i, elem) in elements.iter().enumerate().skip(1) {
                    let elem_ty = self.check_expr(elem);
                    if elem_ty != first_ty {
                        self.errors.push(CompileError::type_error(
                            format!(
                                "array element {} has type '{}', expected '{}'",
                                i, elem_ty, first_ty
                            ),
                            elem.span(),
                        ));
                    }
                }

                NyType::Array {
                    elem: Box::new(first_ty),
                    size: elements.len(),
                }
            }

            // ── Index expression ──────────────────────────────────────
            Expr::Index {
                object,
                index,
                span,
            } => {
                let obj_ty = self.check_expr(object);
                let idx_ty = self.check_expr(index);

                if !idx_ty.is_integer() {
                    self.errors.push(CompileError::type_error(
                        format!("array index must be an integer type, found '{}'", idx_ty),
                        index.span(),
                    ));
                }

                match &obj_ty {
                    NyType::Array { elem, .. } => *elem.clone(),
                    NyType::Pointer(inner) => *inner.clone(),
                    _ => {
                        self.errors.push(CompileError::type_error(
                            format!("cannot index into type '{}'", obj_ty),
                            *span,
                        ));
                        NyType::I32
                    }
                }
            }

            // ── Field access (with auto-deref for pointer-to-struct) ─
            Expr::FieldAccess {
                object,
                field,
                span,
            } => {
                let obj_ty = self.check_expr(object);
                self.resolve_field_access(&obj_ty, field, *span)
            }

            // ── Struct initialization ─────────────────────────────────
            Expr::StructInit { name, fields, span } => {
                if let Some(def_fields) = self.structs.get(name).cloned() {
                    // Check that all provided fields exist and types match
                    for (field_name, field_expr) in fields {
                        let field_ty = self.check_expr(field_expr);
                        if let Some((_, expected_ty)) =
                            def_fields.iter().find(|(n, _)| n == field_name)
                        {
                            if field_ty != *expected_ty {
                                self.errors.push(CompileError::type_error(
                                    format!(
                                        "field '{}' of struct '{}': expected '{}', found '{}'",
                                        field_name, name, expected_ty, field_ty
                                    ),
                                    field_expr.span(),
                                ));
                            }
                        } else {
                            self.errors.push(CompileError::type_error(
                                format!("struct '{}' has no field named '{}'", name, field_name),
                                field_expr.span(),
                            ));
                        }
                    }

                    // Check that all required fields are provided
                    let provided: Vec<&str> = fields.iter().map(|(n, _)| n.as_str()).collect();
                    for (def_name, _) in &def_fields {
                        if !provided.contains(&def_name.as_str()) {
                            self.errors.push(CompileError::type_error(
                                format!(
                                    "missing field '{}' in initializer for struct '{}'",
                                    def_name, name
                                ),
                                *span,
                            ));
                        }
                    }

                    NyType::Struct {
                        name: name.clone(),
                        fields: def_fields,
                    }
                } else {
                    self.errors.push(CompileError::type_error(
                        format!("unknown struct '{}'", name),
                        *span,
                    ));
                    NyType::Unit
                }
            }

            // ── Address-of (&expr) ────────────────────────────────────
            Expr::AddrOf { operand, .. } => {
                let inner_ty = self.check_expr(operand);
                NyType::Pointer(Box::new(inner_ty))
            }

            // ── Dereference (*expr) ───────────────────────────────────
            Expr::Deref { operand, span } => {
                let operand_ty = self.check_expr(operand);
                match operand_ty {
                    NyType::Pointer(inner) => *inner,
                    _ => {
                        self.errors.push(CompileError::type_error(
                            format!("cannot dereference non-pointer type '{}'", operand_ty),
                            *span,
                        ));
                        NyType::I32
                    }
                }
            }

            // ── Method call (desugared to function call with self) ────
            Expr::MethodCall {
                object,
                method,
                args,
                span,
            } => {
                let receiver_ty = self.check_expr(object);
                self.check_method_call(&receiver_ty, method, args, *span)
            }
        }
    }

    /// Resolve a field access, with auto-deref for pointer-to-struct.
    fn resolve_field_access(&mut self, obj_ty: &NyType, field: &str, span: Span) -> NyType {
        match obj_ty {
            NyType::Struct { fields, name, .. } => {
                if let Some(field_ty) = fields.iter().find(|(n, _)| n == field).map(|(_, t)| t) {
                    field_ty.clone()
                } else {
                    self.errors.push(CompileError::type_error(
                        format!("struct '{}' has no field '{}'", name, field),
                        span,
                    ));
                    NyType::I32
                }
            }
            // Auto-deref: if it's a pointer to a struct, dereference and access
            NyType::Pointer(inner) => self.resolve_field_access(inner, field, span),
            _ => {
                self.errors.push(CompileError::type_error(
                    format!("cannot access field '{}' on type '{}'", field, obj_ty),
                    span,
                ));
                NyType::I32
            }
        }
    }

    /// Check a method call: look up a function where the first param is "self"
    /// and its type matches the receiver (or pointer to receiver).
    fn check_method_call(
        &mut self,
        receiver_ty: &NyType,
        method: &str,
        args: &[Expr],
        span: Span,
    ) -> NyType {
        // Look up the function by method name
        let func_info = self.functions.get(method).cloned();
        let param_names = self.function_params.get(method).cloned();

        match (func_info, param_names) {
            (Some((param_types, ret_type)), Some(p_names))
                if !p_names.is_empty() && p_names[0] == "self" =>
            {
                // Validate self parameter type
                let self_param_ty = &param_types[0];
                let self_matches = *self_param_ty == *receiver_ty
                    || *self_param_ty == NyType::Pointer(Box::new(receiver_ty.clone()));

                // Also check if receiver is pointer and self param is the pointee
                let self_matches = self_matches
                    || matches!(receiver_ty, NyType::Pointer(inner) if **inner == *self_param_ty);

                if !self_matches {
                    self.errors.push(CompileError::type_error(
                        format!(
                            "method '{}' expects self of type '{}', found '{}'",
                            method, self_param_ty, receiver_ty
                        ),
                        span,
                    ));
                }

                // The total expected args = param_types.len() - 1 (excluding self)
                let expected_arg_count = param_types.len() - 1;
                if args.len() != expected_arg_count {
                    self.errors.push(CompileError::type_error(
                        format!(
                            "method '{}' expects {} arguments, found {}",
                            method,
                            expected_arg_count,
                            args.len()
                        ),
                        span,
                    ));
                } else {
                    // Type-check remaining arguments (skip self)
                    for (i, (arg, expected_ty)) in
                        args.iter().zip(param_types[1..].iter()).enumerate()
                    {
                        let arg_ty = self.check_expr(arg);
                        if arg_ty != *expected_ty {
                            self.errors.push(CompileError::type_error(
                                format!(
                                    "argument {} of method '{}': expected '{}', found '{}'",
                                    i + 1,
                                    method,
                                    expected_ty,
                                    arg_ty
                                ),
                                arg.span(),
                            ));
                        }
                    }
                }

                ret_type
            }
            _ => {
                // Not a known method -- check args anyway for error recovery
                for arg in args {
                    self.check_expr(arg);
                }
                self.errors.push(CompileError::type_error(
                    format!("no method '{}' found for type '{}'", method, receiver_ty),
                    span,
                ));
                NyType::I32
            }
        }
    }

    fn check_stmt(&mut self, stmt: &Stmt) {
        match stmt {
            // ── Variable declaration ──────────────────────────────────
            Stmt::VarDecl { name, ty, init, .. } => {
                let init_ty = self.check_expr(init);
                if let Some(annotation) = ty {
                    if let Some(declared_ty) = self.resolve_type_annotation(annotation) {
                        if init_ty != declared_ty {
                            self.errors.push(CompileError::type_error(
                                format!("expected '{}', found '{}'", declared_ty, init_ty),
                                init.span(),
                            ));
                        }
                        self.declare(name, declared_ty);
                    } else {
                        self.errors.push(CompileError::type_error(
                            format!("unknown type in annotation"),
                            annotation.span(),
                        ));
                        // Fall back to inferred type
                        self.declare(name, init_ty);
                    }
                } else {
                    // Type inference: no annotation → infer from init expr
                    self.declare(name, init_ty);
                }
            }

            // ── Constant declaration ──────────────────────────────────
            Stmt::ConstDecl {
                name,
                ty,
                value,
                span: _,
            } => {
                let val_ty = self.check_expr(value);
                if let Some(annotation) = ty {
                    if let Some(declared_ty) = self.resolve_type_annotation(annotation) {
                        if val_ty != declared_ty {
                            self.errors.push(CompileError::type_error(
                                format!("expected '{}', found '{}'", declared_ty, val_ty),
                                value.span(),
                            ));
                        }
                        self.declare(name, declared_ty);
                    } else {
                        self.errors.push(CompileError::type_error(
                            "unknown type in annotation",
                            annotation.span(),
                        ));
                        self.declare(name, val_ty);
                    }
                } else {
                    self.declare(name, val_ty);
                }
            }

            // ── Assignment (with AssignTarget variants) ───────────────
            Stmt::Assign {
                target,
                value,
                span,
            } => {
                let val_ty = self.check_expr(value);
                match target {
                    AssignTarget::Var(name) => {
                        if let Some(expected_ty) = self.lookup(name) {
                            if val_ty != expected_ty {
                                self.errors.push(CompileError::type_error(
                                    format!(
                                        "cannot assign '{}' to variable '{}' of type '{}'",
                                        val_ty, name, expected_ty
                                    ),
                                    value.span(),
                                ));
                            }
                        }
                        // Immutability check is done in resolver
                    }
                    AssignTarget::Index(obj_expr, idx_expr) => {
                        let obj_ty = self.check_expr(obj_expr);
                        let idx_ty = self.check_expr(idx_expr);

                        if !idx_ty.is_integer() {
                            self.errors.push(CompileError::type_error(
                                format!("array index must be an integer type, found '{}'", idx_ty),
                                idx_expr.span(),
                            ));
                        }

                        match &obj_ty {
                            NyType::Array { elem, .. } => {
                                if val_ty != **elem {
                                    self.errors.push(CompileError::type_error(
                                        format!(
                                            "cannot assign '{}' to array element of type '{}'",
                                            val_ty, elem
                                        ),
                                        value.span(),
                                    ));
                                }
                            }
                            _ => {
                                self.errors.push(CompileError::type_error(
                                    format!("cannot index into type '{}'", obj_ty),
                                    *span,
                                ));
                            }
                        }
                    }
                    AssignTarget::Field(obj_expr, field_name) => {
                        let obj_ty = self.check_expr(obj_expr);
                        let field_ty = self.resolve_field_access(&obj_ty, field_name, *span);
                        if val_ty != field_ty {
                            self.errors.push(CompileError::type_error(
                                format!(
                                    "cannot assign '{}' to field '{}' of type '{}'",
                                    val_ty, field_name, field_ty
                                ),
                                value.span(),
                            ));
                        }
                    }
                    AssignTarget::Deref(ptr_expr) => {
                        let ptr_ty = self.check_expr(ptr_expr);
                        match &ptr_ty {
                            NyType::Pointer(inner) => {
                                if val_ty != **inner {
                                    self.errors.push(CompileError::type_error(
                                        format!(
                                            "cannot assign '{}' through pointer to '{}'",
                                            val_ty, inner
                                        ),
                                        value.span(),
                                    ));
                                }
                            }
                            _ => {
                                self.errors.push(CompileError::type_error(
                                    format!(
                                        "cannot dereference non-pointer type '{}' in assignment",
                                        ptr_ty
                                    ),
                                    *span,
                                ));
                            }
                        }
                    }
                }
            }

            // ── Expression statement ──────────────────────────────────
            Stmt::ExprStmt { expr, .. } => {
                self.check_expr(expr);
            }

            // ── Return ────────────────────────────────────────────────
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

            // ── While loop ────────────────────────────────────────────
            Stmt::While {
                condition, body, ..
            } => {
                let cond_ty = self.check_expr(condition);
                if cond_ty != NyType::Bool {
                    self.errors.push(CompileError::type_error(
                        format!("while condition must be bool, found '{}'", cond_ty),
                        condition.span(),
                    ));
                }
                self.loop_depth += 1;
                self.check_expr(body);
                self.loop_depth -= 1;
            }

            // ── For-range loop ────────────────────────────────────────
            Stmt::ForRange {
                var,
                start,
                end,
                body,
                span,
                ..
            } => {
                let start_ty = self.check_expr(start);
                let end_ty = self.check_expr(end);

                if !start_ty.is_integer() {
                    self.errors.push(CompileError::type_error(
                        format!(
                            "for-range start must be an integer type, found '{}'",
                            start_ty
                        ),
                        start.span(),
                    ));
                }
                if !end_ty.is_integer() {
                    self.errors.push(CompileError::type_error(
                        format!("for-range end must be an integer type, found '{}'", end_ty),
                        end.span(),
                    ));
                }
                if start_ty != end_ty {
                    self.errors.push(CompileError::type_error(
                        format!(
                            "for-range start and end must have the same type: '{}' vs '{}'",
                            start_ty, end_ty
                        ),
                        *span,
                    ));
                }

                // Declare loop variable in a new scope
                self.push_scope();
                self.declare(var, start_ty);
                self.loop_depth += 1;
                self.check_expr(body);
                self.loop_depth -= 1;
                self.pop_scope();
            }

            // ── Break ─────────────────────────────────────────────────
            Stmt::Break { span } => {
                if self.loop_depth == 0 {
                    self.errors.push(CompileError::syntax(
                        "'break' used outside of a loop",
                        *span,
                    ));
                }
            }

            // ── Continue ──────────────────────────────────────────────
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
}
