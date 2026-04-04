use std::collections::HashMap;

use crate::common::{CompileError, NyType, Span};
use crate::parser::ast::*;
use crate::semantic::resolver::ResolvedInfo;

pub struct TypeChecker {
    scopes: Vec<HashMap<String, NyType>>,
    functions: HashMap<String, (Vec<NyType>, NyType)>,
    /// Struct definitions: struct_name → fields
    structs: HashMap<String, Vec<(String, NyType)>>,
    /// Enum definitions: enum_name → variants with payload types
    enums: HashMap<String, Vec<(String, Vec<NyType>)>>,
    /// Function parameter names (needed for method call self-parameter detection)
    function_params: HashMap<String, Vec<String>>,
    current_return_type: NyType,
    errors: Vec<CompileError>,
    loop_depth: usize,
    /// Trait definitions: trait_name → (method_sigs, span)
    traits: HashMap<String, (Vec<(String, Vec<NyType>, NyType)>, Span)>,
    /// Type aliases: alias_name → resolved NyType
    type_aliases: HashMap<String, NyType>,
}

impl TypeChecker {
    pub fn new(resolved: &ResolvedInfo) -> Self {
        let functions = resolved
            .functions
            .iter()
            .map(|(name, (params, ret, _))| (name.clone(), (params.clone(), ret.clone())))
            .collect();

        let structs = resolved.structs.clone();
        let enums = resolved.enums.clone();

        Self {
            scopes: vec![HashMap::new()],
            functions,
            structs,
            enums,
            function_params: HashMap::new(),
            current_return_type: NyType::Unit,
            errors: Vec::new(),
            loop_depth: 0,
            traits: HashMap::new(),
            type_aliases: resolved.type_aliases.clone(),
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

    /// Find the closest match from a list of candidates for "did you mean?" suggestions.
    fn suggest_similar(name: &str, candidates: &[&str]) -> Option<String> {
        let mut best: Option<(&str, usize)> = None;
        let max_dist = if name.len() <= 2 { 1 } else { 2 };
        for &c in candidates {
            let dist = crate::common::edit_distance(name, c);
            if dist > 0 && dist <= max_dist && best.as_ref().map_or(true, |(_, d)| dist < *d) {
                best = Some((c, dist));
            }
        }
        best.map(|(s, _)| s.to_string())
    }

    /// Resolve a TypeAnnotation into a NyType, using struct definitions when needed.
    fn resolve_type_annotation(&self, ann: &TypeAnnotation) -> Option<NyType> {
        match ann {
            TypeAnnotation::Named { name, .. } => {
                // Check primitive types first
                if let Some(ty) = NyType::from_name(name) {
                    return Some(ty);
                }
                // Check Vec<StructName> pattern — from_name returns None for struct inners
                if let Some(inner) = name.strip_prefix("Vec<").and_then(|s| s.strip_suffix('>')) {
                    // Try to resolve inner as struct
                    if let Some(fields) = self.structs.get(inner) {
                        return Some(NyType::Vec(Box::new(NyType::Struct {
                            name: inner.to_string(),
                            fields: fields.clone(),
                        })));
                    }
                    // Try as enum
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
                // Check struct types
                if let Some(fields) = self.structs.get(name) {
                    return Some(NyType::Struct {
                        name: name.clone(),
                        fields: fields.clone(),
                    });
                }
                // Check enum types
                if let Some(variant_defs) = self.enums.get(name) {
                    return Some(NyType::Enum {
                        name: name.clone(),
                        variants: variant_defs.clone(),
                    });
                }
                // "unit" or "()" convention
                if name == "()" {
                    return Some(NyType::Unit);
                }
                // Type aliases
                if let Some(ty) = self.type_aliases.get(name) {
                    return Some(ty.clone());
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
            TypeAnnotation::Tuple { elements, .. } => {
                let mut resolved = Vec::new();
                for elem in elements {
                    resolved.push(self.resolve_type_annotation(elem)?);
                }
                Some(NyType::Tuple(resolved))
            }
            TypeAnnotation::Slice { elem, .. } => {
                let elem_ty = self.resolve_type_annotation(elem)?;
                Some(NyType::Slice(Box::new(elem_ty)))
            }
            TypeAnnotation::Function { params, ret, .. } => {
                let mut param_types = Vec::new();
                for p in params {
                    param_types.push(self.resolve_type_annotation(p)?);
                }
                let ret_ty = self.resolve_type_annotation(ret)?;
                Some(NyType::Function {
                    params: param_types,
                    ret: Box::new(ret_ty),
                })
            }
            TypeAnnotation::DynTrait { trait_name, .. } => {
                Some(NyType::DynTrait(trait_name.clone()))
            }
            TypeAnnotation::Optional { inner, .. } => {
                let inner_ty = self.resolve_type_annotation(inner)?;
                Some(NyType::Optional(Box::new(inner_ty)))
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
            // Register impl block methods with both original and qualified names
            if let Item::ImplBlock {
                type_name, methods, ..
            } = item
            {
                for method in methods {
                    if let Item::FunctionDef { name, params, .. } = method {
                        let param_names: Vec<String> =
                            params.iter().map(|p| p.name.clone()).collect();
                        let qualified_name = format!("{}_{}", type_name, name);
                        checker
                            .function_params
                            .insert(qualified_name, param_names.clone());
                        checker.function_params.insert(name.clone(), param_names);
                    }
                }
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
                is_async,
                ..
            } => {
                if *is_async {
                    eprintln!(
                        "warning: 'async fn' is deprecated — use 'go fn()' + channels instead"
                    );
                }
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
            Item::EnumDef { .. } => {
                // Enum definitions are already registered in ResolvedInfo.
                // No additional type checking needed at definition site.
            }
            Item::ImplBlock {
                type_name,
                trait_name,
                methods,
                span,
            } => {
                // Check trait conformance if this is `impl Trait for Type`
                if let Some(tname) = trait_name {
                    if let Some((required_sigs, _)) = self.traits.get(tname).cloned() {
                        let impl_method_names: Vec<String> = methods
                            .iter()
                            .filter_map(|m| match m {
                                Item::FunctionDef { name, .. } => Some(name.clone()),
                                _ => None,
                            })
                            .collect();
                        for (req_name, _, _) in &required_sigs {
                            if !impl_method_names.contains(req_name) {
                                self.errors.push(CompileError::type_error(
                                    format!(
                                        "impl '{}' for '{}' is missing method '{}'",
                                        tname, type_name, req_name
                                    ),
                                    *span,
                                ));
                            }
                        }
                    } else {
                        self.errors.push(CompileError::type_error(
                            format!("undeclared trait '{}'", tname),
                            *span,
                        ));
                    }
                }
                // Warn about operator overloading on non-numeric types
                if trait_name.is_none() {
                    let op_names = [
                        "add", "sub", "mul", "div", "eq", "ne", "lt", "gt", "le", "ge",
                    ];
                    for method in methods.iter() {
                        if let Item::FunctionDef { name, .. } = method {
                            if op_names.contains(&name.as_str()) {
                                eprintln!(
                                    "warning: operator overloading '{}' on '{}' can reduce readability — consider a named method",
                                    name, type_name
                                );
                            }
                        }
                    }
                }
                for method in methods {
                    self.check_item(method);
                }
            }
            Item::Use { .. } => {
                // Already resolved at compile time
            }
            Item::ExternBlock { .. } => {
                // Extern functions registered in resolver pass
            }
            Item::TraitDef {
                name,
                methods,
                span,
            } => {
                // Register trait for conformance checking
                let sigs: Vec<(String, Vec<NyType>, NyType)> = methods
                    .iter()
                    .filter_map(|sig| {
                        let param_types: Vec<NyType> = sig
                            .params
                            .iter()
                            .filter_map(|p| self.resolve_type_annotation(&p.ty))
                            .collect();
                        let ret_ty = self
                            .resolve_type_annotation(&sig.return_type)
                            .unwrap_or(NyType::Unit);
                        Some((sig.name.clone(), param_types, ret_ty))
                    })
                    .collect();
                self.traits.insert(name.clone(), (sigs, *span));
            }
            Item::TypeAlias { .. } => {
                // Already registered in resolver pass
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
                LitValue::Nil => NyType::Pointer(Box::new(NyType::U8)),
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
                    BinOp::Add => {
                        // SIMD vector arithmetic
                        if lhs_ty.is_simd() && lhs_ty == rhs_ty {
                            return lhs_ty;
                        }
                        // String concatenation: str + str → str
                        if lhs_ty == NyType::Str && rhs_ty == NyType::Str {
                            NyType::Str
                        } else if lhs_ty.is_pointer() && rhs_ty.is_integer() {
                            lhs_ty // pointer arithmetic: ptr + int → ptr
                        } else if let NyType::Struct { name, .. } = &lhs_ty {
                            // Operator overloading: check for TypeName_add function
                            let method = format!("{}_add", name);
                            if self.functions.contains_key(&method) {
                                return lhs_ty;
                            }
                            self.errors.push(CompileError::type_error(
                                format!("no 'add' method defined for struct '{}'", name),
                                *span,
                            ));
                            lhs_ty
                        } else {
                            if !lhs_ty.is_numeric() {
                                self.errors.push(CompileError::type_error(
                                    format!(
                                        "arithmetic operator requires numeric type, found '{}'",
                                        lhs_ty
                                    ),
                                    lhs.span(),
                                ));
                            }
                            if lhs_ty != rhs_ty && !(lhs_ty.is_numeric() && rhs_ty.is_numeric()) {
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
                    }
                    BinOp::Sub | BinOp::Mul | BinOp::Div | BinOp::Mod => {
                        // SIMD vector arithmetic
                        if lhs_ty.is_simd() && lhs_ty == rhs_ty {
                            return lhs_ty;
                        }
                        // pointer - int → pointer
                        if *op == BinOp::Sub && lhs_ty.is_pointer() && rhs_ty.is_integer() {
                            return lhs_ty;
                        }
                        // Operator overloading for structs
                        if let NyType::Struct { name, .. } = &lhs_ty {
                            let op_name = match op {
                                BinOp::Sub => "sub",
                                BinOp::Mul => "mul",
                                BinOp::Div => "div",
                                _ => "mod",
                            };
                            let method = format!("{}_{}", name, op_name);
                            if self.functions.contains_key(&method) {
                                return lhs_ty;
                            }
                        }
                        if !lhs_ty.is_numeric() {
                            self.errors.push(CompileError::type_error(
                                format!(
                                    "arithmetic operator requires numeric type, found '{}'",
                                    lhs_ty
                                ),
                                lhs.span(),
                            ));
                        }
                        if lhs_ty != rhs_ty && !(lhs_ty.is_numeric() && rhs_ty.is_numeric()) {
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
                        let nil_cmp = (lhs_ty.is_pointer() && rhs_ty.is_pointer())
                            || matches!(
                                (&lhs_ty, &rhs_ty),
                                (NyType::DynTrait(_), NyType::Pointer(_))
                            )
                            || matches!(
                                (&lhs_ty, &rhs_ty),
                                (NyType::Pointer(_), NyType::DynTrait(_))
                            )
                            || matches!(
                                (&lhs_ty, &rhs_ty),
                                (NyType::Optional(_), NyType::Pointer(_))
                            )
                            || matches!(
                                (&lhs_ty, &rhs_ty),
                                (NyType::Pointer(_), NyType::Optional(_))
                            );
                        if lhs_ty != rhs_ty
                            && !(lhs_ty.is_numeric() && rhs_ty.is_numeric())
                            && !nil_cmp
                        {
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
                    BinOp::BitAnd | BinOp::BitOr | BinOp::BitXor => {
                        if !lhs_ty.is_integer() {
                            self.errors.push(CompileError::type_error(
                                format!(
                                    "bitwise operator requires integer type, found '{}'",
                                    lhs_ty
                                ),
                                lhs.span(),
                            ));
                        }
                        if lhs_ty != rhs_ty {
                            self.errors.push(CompileError::type_error(
                                format!(
                                    "type mismatch in bitwise op: '{}' and '{}'",
                                    lhs_ty, rhs_ty
                                ),
                                *span,
                            ));
                        }
                        lhs_ty
                    }
                    BinOp::Shl | BinOp::Shr => {
                        if !lhs_ty.is_integer() {
                            self.errors.push(CompileError::type_error(
                                format!("shift operator requires integer type, found '{}'", lhs_ty),
                                lhs.span(),
                            ));
                        }
                        if !rhs_ty.is_integer() {
                            self.errors.push(CompileError::type_error(
                                format!("shift amount must be integer, found '{}'", rhs_ty),
                                rhs.span(),
                            ));
                        }
                        lhs_ty
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
                    UnaryOp::BitNot => {
                        if !operand_ty.is_integer() {
                            self.errors.push(CompileError::type_error(
                                format!(
                                    "bitwise NOT requires integer type, found '{}'",
                                    operand_ty
                                ),
                                *span,
                            ));
                        }
                        operand_ty
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

                // Built-in alloc(size) -> *i8 (generic pointer)
                if callee == "alloc" {
                    if args.len() != 1 {
                        self.errors.push(CompileError::type_error(
                            format!("'alloc' expects 1 argument, found {}", args.len()),
                            *span,
                        ));
                    } else {
                        let arg_ty = self.check_expr(&args[0]);
                        if !arg_ty.is_integer() {
                            self.errors.push(CompileError::type_error(
                                format!("'alloc' expects integer size, found '{}'", arg_ty),
                                args[0].span(),
                            ));
                        }
                    }
                    return NyType::Pointer(Box::new(NyType::U8));
                }

                // Built-in free(ptr) -> Unit
                if callee == "free" {
                    if args.len() != 1 {
                        self.errors.push(CompileError::type_error(
                            format!("'free' expects 1 argument, found {}", args.len()),
                            *span,
                        ));
                    } else {
                        let arg_ty = self.check_expr(&args[0]);
                        if !arg_ty.is_pointer() {
                            self.errors.push(CompileError::type_error(
                                format!("'free' expects a pointer, found '{}'", arg_ty),
                                args[0].span(),
                            ));
                        }
                    }
                    return NyType::Unit;
                }

                // Built-in gc_alloc(size) -> *u8 (GC-managed allocation)
                if callee == "gc_alloc" {
                    if args.len() != 1 {
                        self.errors.push(CompileError::type_error(
                            format!("'gc_alloc' expects 1 argument, found {}", args.len()),
                            *span,
                        ));
                    } else {
                        let arg_ty = self.check_expr(&args[0]);
                        if !arg_ty.is_integer() {
                            self.errors.push(CompileError::type_error(
                                format!("'gc_alloc' expects integer size, found '{}'", arg_ty),
                                args[0].span(),
                            ));
                        }
                    }
                    return NyType::Pointer(Box::new(NyType::U8));
                }

                // Built-in gc_collect() / gc_stats() -> unit
                if callee == "gc_collect" || callee == "gc_stats" {
                    return NyType::Unit;
                }

                // Built-in gc_bytes_allocated() / gc_collection_count() -> i64
                if callee == "gc_bytes_allocated" || callee == "gc_collection_count" {
                    return NyType::I64;
                }

                // Built-in error_new(message) -> i32 (error code)
                if callee == "error_new" {
                    for arg in args {
                        self.check_expr(arg);
                    }
                    return NyType::I32;
                }

                // Built-in error_message(code) -> str
                if callee == "error_message" {
                    for arg in args {
                        self.check_expr(arg);
                    }
                    return NyType::Str;
                }

                // Built-in error_trace(code) -> str
                if callee == "error_trace" {
                    for arg in args {
                        self.check_expr(arg);
                    }
                    return NyType::Str;
                }

                // Built-in chan_new(capacity) -> chan<T>
                if callee == "chan_new" {
                    if args.len() != 1 {
                        self.errors.push(CompileError::type_error(
                            format!("'chan_new' expects 1 argument, found {}", args.len()),
                            *span,
                        ));
                    } else {
                        self.check_expr(&args[0]);
                    }
                    return NyType::Chan(Box::new(NyType::I32)); // refined by type annotation
                }

                // Built-in fopen(path, mode) -> *u8 (FILE pointer)
                if callee == "fopen" {
                    for arg in args {
                        self.check_expr(arg);
                    }
                    return NyType::Pointer(Box::new(NyType::U8));
                }

                // Built-in fclose(fp) -> i32
                if callee == "fclose" {
                    for arg in args {
                        self.check_expr(arg);
                    }
                    return NyType::I32;
                }

                // Built-in fwrite_str(fp, str) -> i32
                if callee == "fwrite_str" {
                    for arg in args {
                        self.check_expr(arg);
                    }
                    return NyType::I32;
                }

                // Built-in fread_byte(fp) -> i32 (-1 on EOF)
                if callee == "fread_byte" {
                    for arg in args {
                        self.check_expr(arg);
                    }
                    return NyType::I32;
                }

                // Built-in exit(code) -> Unit
                if callee == "exit" {
                    for arg in args {
                        self.check_expr(arg);
                    }
                    return NyType::Unit;
                }

                // Arena builtins
                if callee == "arena_new" {
                    for arg in args {
                        self.check_expr(arg);
                    }
                    return NyType::Pointer(Box::new(NyType::U8));
                }
                if callee == "arena_alloc" {
                    for arg in args {
                        self.check_expr(arg);
                    }
                    return NyType::Pointer(Box::new(NyType::U8));
                }
                if callee == "arena_free" || callee == "arena_reset" {
                    for arg in args {
                        self.check_expr(arg);
                    }
                    return NyType::Unit;
                }
                if callee == "arena_bytes_used" {
                    for arg in args {
                        self.check_expr(arg);
                    }
                    return NyType::I64;
                }

                // Built-in map_new() -> *u8 (opaque HashMap pointer)
                if callee == "map_new" {
                    return NyType::Pointer(Box::new(NyType::U8));
                }
                if callee == "map_insert" {
                    for arg in args {
                        self.check_expr(arg);
                    }
                    return NyType::Unit;
                }
                if callee == "map_get" {
                    for arg in args {
                        self.check_expr(arg);
                    }
                    return NyType::I32;
                }
                if callee == "map_contains" {
                    for arg in args {
                        self.check_expr(arg);
                    }
                    return NyType::Bool;
                }
                if callee == "map_len" {
                    for arg in args {
                        self.check_expr(arg);
                    }
                    return NyType::I64;
                }

                // Built-in vec_new() -> Vec<T>
                // Type is determined by the variable's type annotation at the call site
                // Default to Vec<i32> when no context is available
                if callee == "vec_new" {
                    return NyType::Vec(Box::new(NyType::I32));
                }

                // Built-in vec_push(v: Vec<T>, val: T) -> Unit
                if callee == "vec_push" {
                    for arg in args {
                        self.check_expr(arg);
                    }
                    return NyType::Unit;
                }

                // Built-in vec_len(v: Vec<T>) -> i64
                if callee == "vec_len" {
                    for arg in args {
                        self.check_expr(arg);
                    }
                    return NyType::I64;
                }

                // Built-in vec_get(v: Vec<T>, idx: i32) -> T
                if callee == "vec_get" {
                    if !args.is_empty() {
                        let vec_ty = self.check_expr(&args[0]);
                        if let NyType::Vec(elem) = &vec_ty {
                            for arg in args.iter().skip(1) {
                                self.check_expr(arg);
                            }
                            return *elem.clone();
                        }
                    }
                    for arg in args {
                        self.check_expr(arg);
                    }
                    return NyType::I32;
                }

                // Built-in read_line() -> str (reads a line from stdin)
                if callee == "read_line" {
                    return NyType::Str;
                }

                // Built-in str_to_int(s: str) -> i32
                if callee == "str_to_int" {
                    if args.len() == 1 {
                        let arg_ty = self.check_expr(&args[0]);
                        if arg_ty != NyType::Str {
                            self.errors.push(CompileError::type_error(
                                format!("'str_to_int' expects str, found '{}'", arg_ty),
                                args[0].span(),
                            ));
                        }
                    }
                    return NyType::I32;
                }

                // Built-in int_to_str(n: i32) -> str
                if callee == "int_to_str" {
                    if args.len() == 1 {
                        let arg_ty = self.check_expr(&args[0]);
                        if !arg_ty.is_integer() {
                            self.errors.push(CompileError::type_error(
                                format!("'int_to_str' expects integer, found '{}'", arg_ty),
                                args[0].span(),
                            ));
                        }
                    }
                    return NyType::Str;
                }

                // float_to_str / read_file -> str
                if callee == "float_to_str" || callee == "read_file" {
                    for arg in args {
                        self.check_expr(arg);
                    }
                    return NyType::Str;
                }

                // str_to_float -> f64, write_file -> i32
                if callee == "str_to_float" {
                    for arg in args {
                        self.check_expr(arg);
                    }
                    return NyType::F64;
                }
                // Tensor builtins
                if callee.starts_with("tensor_") {
                    for arg in args {
                        self.check_expr(arg);
                    }
                    return match callee.as_str() {
                        "tensor_zeros" | "tensor_ones" | "tensor_fill" | "tensor_rand"
                        | "tensor_clone" | "tensor_add" | "tensor_sub" | "tensor_mul"
                        | "tensor_scale" | "tensor_matmul" | "tensor_transpose" => {
                            NyType::Pointer(Box::new(NyType::U8))
                        }
                        "tensor_get" | "tensor_sum" | "tensor_max" | "tensor_min"
                        | "tensor_dot" | "tensor_norm" => NyType::F64,
                        "tensor_rows" | "tensor_cols" => NyType::I64,
                        _ => NyType::Unit,
                    };
                }

                // hmap_new() → HashMap<str, i32> (annotation overrides)
                if callee == "hmap_new" {
                    return NyType::HashMap(Box::new(NyType::Str), Box::new(NyType::I32));
                }

                // String→String Map
                if callee == "smap_new" {
                    return NyType::Pointer(Box::new(NyType::U8));
                }
                if callee == "smap_insert" || callee == "smap_free" {
                    for arg in args {
                        self.check_expr(arg);
                    }
                    return NyType::Unit;
                }
                if callee == "smap_get" {
                    for arg in args {
                        self.check_expr(arg);
                    }
                    return NyType::Str;
                }
                if callee == "smap_contains" {
                    for arg in args {
                        self.check_expr(arg);
                    }
                    return NyType::Bool;
                }
                if callee == "smap_len" {
                    for arg in args {
                        self.check_expr(arg);
                    }
                    return NyType::I64;
                }
                // map_key_at(m, index) -> str
                if callee == "map_key_at" {
                    for arg in args {
                        self.check_expr(arg);
                    }
                    return NyType::Str;
                }
                // JSON builtins
                if callee == "json_parse" || callee == "json_arr_get" {
                    for arg in args {
                        self.check_expr(arg);
                    }
                    return NyType::Pointer(Box::new(NyType::U8));
                }
                if callee == "json_get_str" {
                    for arg in args {
                        self.check_expr(arg);
                    }
                    return NyType::Str;
                }
                if callee == "json_get_int" || callee == "json_type" || callee == "json_len" {
                    for arg in args {
                        self.check_expr(arg);
                    }
                    return NyType::I32;
                }
                if callee == "json_get_float" {
                    for arg in args {
                        self.check_expr(arg);
                    }
                    return NyType::F64;
                }
                if callee == "json_get_bool" {
                    for arg in args {
                        self.check_expr(arg);
                    }
                    return NyType::Bool;
                }
                if callee == "json_free" {
                    for arg in args {
                        self.check_expr(arg);
                    }
                    return NyType::Unit;
                }

                if callee == "write_file" || callee == "remove_file" {
                    for arg in args {
                        self.check_expr(arg);
                    }
                    return NyType::I32;
                }

                // Math builtins -> f64
                if matches!(
                    callee.as_str(),
                    "sqrt" | "sin" | "cos" | "floor" | "ceil" | "fabs" | "log" | "exp" | "pow"
                ) {
                    for arg in args {
                        self.check_expr(arg);
                    }
                    return NyType::F64;
                }

                // SIMD builtins
                if callee == "simd_splat_f32x4" || callee == "simd_load_f32x4" {
                    for arg in args {
                        self.check_expr(arg);
                    }
                    return NyType::Simd {
                        elem: Box::new(NyType::F32),
                        lanes: 4,
                    };
                }
                if callee == "simd_splat_f32x8" || callee == "simd_load_f32x8" {
                    for arg in args {
                        self.check_expr(arg);
                    }
                    return NyType::Simd {
                        elem: Box::new(NyType::F32),
                        lanes: 8,
                    };
                }
                if callee == "simd_reduce_add_f32" {
                    for arg in args {
                        self.check_expr(arg);
                    }
                    return NyType::F32;
                }
                if callee == "simd_store_f32x4" || callee == "simd_store_f32x8" {
                    for arg in args {
                        self.check_expr(arg);
                    }
                    return NyType::Unit;
                }

                // Channel builtins
                if callee == "channel_new" {
                    for arg in args {
                        self.check_expr(arg);
                    }
                    return NyType::Pointer(Box::new(NyType::U8));
                }
                if callee == "channel_send" || callee == "channel_close" {
                    for arg in args {
                        self.check_expr(arg);
                    }
                    return NyType::Unit;
                }
                if callee == "channel_recv" {
                    for arg in args {
                        self.check_expr(arg);
                    }
                    return NyType::I32;
                }
                // Pool builtins
                if callee == "pool_new" {
                    for arg in args {
                        self.check_expr(arg);
                    }
                    return NyType::Pointer(Box::new(NyType::U8));
                }
                if callee == "pool_submit" || callee == "pool_wait" || callee == "pool_free" {
                    for arg in args {
                        self.check_expr(arg);
                    }
                    return NyType::Unit;
                }
                // Parallel iterator builtins
                if callee == "par_map" {
                    for arg in args {
                        self.check_expr(arg);
                    }
                    return NyType::Unit;
                }
                if callee == "par_reduce" {
                    for arg in args {
                        self.check_expr(arg);
                    }
                    return NyType::I32;
                }

                // Built-in to_str(any) -> str
                if callee == "to_str" {
                    for arg in args {
                        self.check_expr(arg);
                    }
                    return NyType::Str;
                }

                // Built-in thread_spawn(fn) -> i64
                if callee == "thread_spawn" {
                    for arg in args {
                        self.check_expr(arg);
                    }
                    return NyType::I64;
                }

                // Built-in thread_join(handle) -> ()
                if callee == "thread_join" {
                    for arg in args {
                        self.check_expr(arg);
                    }
                    return NyType::Unit;
                }

                // Built-in sleep_ms(milliseconds) -> Unit
                if callee == "sleep_ms" {
                    for arg in args {
                        self.check_expr(arg);
                    }
                    return NyType::Unit;
                }

                // Built-in sizeof(expr) -> i64
                // clock_ms() -> i64
                if callee == "clock_ms" {
                    return NyType::I64;
                }

                // str_split_count(str, delim) -> i32
                if callee == "str_split_count" {
                    for arg in args {
                        self.check_expr(arg);
                    }
                    return NyType::I32;
                }

                // str_split_get(str, delim, index) -> str
                if callee == "str_split_get" {
                    for arg in args {
                        self.check_expr(arg);
                    }
                    return NyType::Str;
                }

                if callee == "sizeof" {
                    if args.len() != 1 {
                        self.errors.push(CompileError::type_error(
                            format!("'sizeof' expects 1 argument, found {}", args.len()),
                            *span,
                        ));
                    } else {
                        self.check_expr(&args[0]);
                    }
                    return NyType::I64;
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
                    NyType::Slice(elem) => *elem.clone(),
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

            // ── Range index (arr[start..end] → slice) ─────────────────
            Expr::RangeIndex {
                object,
                start,
                end,
                span,
            } => {
                let obj_ty = self.check_expr(object);
                let start_ty = self.check_expr(start);
                let end_ty = self.check_expr(end);
                if !start_ty.is_integer() {
                    self.errors.push(CompileError::type_error(
                        format!("slice start must be integer, found '{}'", start_ty),
                        start.span(),
                    ));
                }
                if !end_ty.is_integer() {
                    self.errors.push(CompileError::type_error(
                        format!("slice end must be integer, found '{}'", end_ty),
                        end.span(),
                    ));
                }
                match &obj_ty {
                    NyType::Array { elem, .. } => NyType::Slice(elem.clone()),
                    NyType::Slice(elem) => NyType::Slice(elem.clone()),
                    _ => {
                        self.errors.push(CompileError::type_error(
                            format!("cannot slice type '{}'", obj_ty),
                            *span,
                        ));
                        NyType::Slice(Box::new(NyType::I32))
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
                            let candidates: Vec<&str> =
                                def_fields.iter().map(|(n, _)| n.as_str()).collect();
                            let mut err = CompileError::type_error(
                                format!("struct '{}' has no field named '{}'", name, field_name),
                                field_expr.span(),
                            );
                            if let Some(s) = Self::suggest_similar(field_name, &candidates) {
                                err = err.with_note(format!("did you mean '{}'?", s));
                            }
                            self.errors.push(err);
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

            // ── New (GC-managed heap allocation) ──────────────────────
            Expr::New { name, fields, span } => {
                if let Some(def_fields) = self.structs.get(name).cloned() {
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
                    // new returns *Struct (pointer to GC-managed struct)
                    NyType::Pointer(Box::new(NyType::Struct {
                        name: name.clone(),
                        fields: def_fields,
                    }))
                } else {
                    self.errors.push(CompileError::type_error(
                        format!("unknown struct '{}'", name),
                        *span,
                    ));
                    NyType::Pointer(Box::new(NyType::Unit))
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

            Expr::Cast {
                expr,
                target_type,
                span,
            } => {
                let source_ty = self.check_expr(expr);
                let target_ty = self
                    .resolve_type_annotation(target_type)
                    .unwrap_or(NyType::I32);

                let valid = match (&source_ty, &target_ty) {
                    (s, t) if s == t => true,
                    (s, t) if s.is_numeric() && t.is_numeric() => true,
                    (NyType::Bool, t) if t.is_integer() => true,
                    _ => false,
                };

                if !valid {
                    self.errors.push(CompileError::type_error(
                        format!("cannot cast '{}' to '{}'", source_ty, target_ty),
                        *span,
                    ));
                }

                target_ty
            }

            // ── Enum variant ─────────────────────────────────────────
            Expr::EnumVariant {
                enum_name,
                variant,
                args,
                span,
            } => {
                if let Some(variants) = self.enums.get(enum_name).cloned() {
                    // Check args match the variant's payload types
                    if let Some((_, payload_types)) =
                        variants.iter().find(|(name, _)| name == variant)
                    {
                        if args.len() != payload_types.len() {
                            self.errors.push(CompileError::type_error(
                                format!(
                                    "variant '{}::{}' expects {} arguments, found {}",
                                    enum_name,
                                    variant,
                                    payload_types.len(),
                                    args.len()
                                ),
                                *span,
                            ));
                        } else {
                            for (i, (arg, expected_ty)) in
                                args.iter().zip(payload_types.iter()).enumerate()
                            {
                                let arg_ty = self.check_expr(arg);
                                if arg_ty != *expected_ty {
                                    self.errors.push(CompileError::type_error(
                                        format!(
                                            "argument {} of '{}::{}': expected '{}', found '{}'",
                                            i + 1,
                                            enum_name,
                                            variant,
                                            expected_ty,
                                            arg_ty
                                        ),
                                        arg.span(),
                                    ));
                                }
                            }
                        }
                    }
                    NyType::Enum {
                        name: enum_name.clone(),
                        variants,
                    }
                } else {
                    self.errors.push(CompileError::type_error(
                        format!("unknown enum '{}'", enum_name),
                        *span,
                    ));
                    NyType::I32
                }
            }

            // ── Match expression ─────────────────────────────────────
            Expr::Match {
                subject,
                arms,
                span,
            } => {
                let subject_ty = self.check_expr(subject);

                // Check each arm's pattern against subject type and collect arm body types
                let mut arm_types: Vec<NyType> = Vec::new();
                let mut covered_variants: Vec<String> = Vec::new();
                let mut has_wildcard = false;

                for arm in arms {
                    match &arm.pattern {
                        Pattern::EnumVariant {
                            enum_name,
                            variant,
                            bindings,
                            span: pat_span,
                        } => {
                            // Verify subject is the same enum type
                            match &subject_ty {
                                NyType::Enum { name, variants } => {
                                    if name != enum_name {
                                        self.errors.push(CompileError::type_error(
                                            format!(
                                                "pattern enum '{}' does not match subject enum '{}'",
                                                enum_name, name
                                            ),
                                            *pat_span,
                                        ));
                                    } else if let Some((_, payload)) =
                                        variants.iter().find(|(n, _)| n == variant)
                                    {
                                        covered_variants.push(variant.clone());
                                        // Declare bindings in scope for arm body
                                        if !bindings.is_empty() {
                                            self.push_scope();
                                            for (i, binding) in bindings.iter().enumerate() {
                                                let ty =
                                                    payload.get(i).cloned().unwrap_or(NyType::I32);
                                                self.declare(binding, ty);
                                            }
                                            let body_ty = self.check_expr(&arm.body);
                                            arm_types.push(body_ty);
                                            self.pop_scope();
                                            continue;
                                        }
                                    } else {
                                        self.errors.push(CompileError::type_error(
                                            format!(
                                                "enum '{}' has no variant '{}'",
                                                enum_name, variant
                                            ),
                                            *pat_span,
                                        ));
                                    }
                                }
                                _ => {
                                    self.errors.push(CompileError::type_error(
                                        format!(
                                            "cannot match enum pattern against non-enum type '{}'",
                                            subject_ty
                                        ),
                                        *pat_span,
                                    ));
                                }
                            }
                        }
                        Pattern::IntLit(_, pat_span) => {
                            if !subject_ty.is_integer() {
                                self.errors.push(CompileError::type_error(
                                    format!(
                                        "cannot match integer pattern against type '{}'",
                                        subject_ty
                                    ),
                                    *pat_span,
                                ));
                            }
                        }
                        Pattern::Wildcard(_) => {
                            has_wildcard = true;
                        }
                        Pattern::OptionalBind { .. } => {
                            // Handled by IfLet, not match
                        }
                    }

                    let body_ty = self.check_expr(&arm.body);
                    arm_types.push(body_ty);
                }

                // Check all arm bodies return the same type
                let result_ty = if let Some(first) = arm_types.first() {
                    for (i, arm_ty) in arm_types.iter().enumerate().skip(1) {
                        if arm_ty != first {
                            self.errors.push(CompileError::type_error(
                                format!(
                                    "match arm {} has type '{}', expected '{}'",
                                    i, arm_ty, first
                                ),
                                arms[i].body.span(),
                            ));
                        }
                    }
                    first.clone()
                } else {
                    NyType::Unit
                };

                // Exhaustiveness check for enums
                if let NyType::Enum { name, variants } = &subject_ty {
                    if !has_wildcard {
                        for (variant_name, _) in variants {
                            if !covered_variants.contains(variant_name) {
                                self.errors.push(
                                    CompileError::type_error(
                                        format!(
                                            "non-exhaustive match: variant '{}::{}' not covered",
                                            name, variant_name
                                        ),
                                        *span,
                                    )
                                    .with_note("consider adding a wildcard arm: _ => { ... }"),
                                );
                            }
                        }
                    }
                }

                result_ty
            }

            // ── Tuple literal ────────────────────────────────────────
            Expr::TupleLit { elements, .. } => {
                let elem_types: Vec<NyType> = elements.iter().map(|e| self.check_expr(e)).collect();
                NyType::Tuple(elem_types)
            }

            // ── Try (?) operator ─────────────────────────────────────
            Expr::Try { operand, span } => {
                let operand_ty = self.check_expr(operand);
                match &operand_ty {
                    NyType::Enum { variants, name } => {
                        if variants.len() < 2 {
                            self.errors.push(CompileError::type_error(
                                format!(
                                    "'?' requires enum with at least 2 variants, '{}' has {}",
                                    name,
                                    variants.len()
                                ),
                                *span,
                            ));
                            NyType::I32
                        } else {
                            // First variant is "success" — extract its first payload type
                            let (_, success_payload) = &variants[0];
                            if success_payload.is_empty() {
                                NyType::Unit
                            } else {
                                success_payload[0].clone()
                            }
                        }
                    }
                    _ => {
                        self.errors.push(CompileError::type_error(
                            format!("'?' requires enum type, found '{}'", operand_ty),
                            *span,
                        ));
                        NyType::I32
                    }
                }
            }

            // ── Await ────────────────────────────────────────────────
            Expr::Await { future, .. } => {
                eprintln!("warning: 'await' is deprecated — use 'go' + channels instead");
                let ft = self.check_expr(future);
                match ft {
                    NyType::Future(inner) => *inner,
                    _ => ft,
                }
            }

            // ── Null coalescing (??) ──────────────────────────────────
            Expr::NullCoalesce { value, default, .. } => {
                let val_ty = self.check_expr(value);
                self.check_expr(default);
                // Unwrap optional: ?T → T
                match val_ty {
                    NyType::Optional(inner) => *inner,
                    _ => val_ty,
                }
            }

            // ── Go (goroutine) ───────────────────────────────────────
            Expr::Go { call, .. } => {
                self.check_expr(call);
                NyType::Unit
            }

            // ── Lambda ───────────────────────────────────────────────
            Expr::Lambda {
                params,
                return_type,
                body,
                ..
            } => {
                let mut param_types = Vec::new();
                self.push_scope();
                for p in params {
                    if let Some(ty) = self.resolve_type_annotation(&p.ty) {
                        self.declare(&p.name, ty.clone());
                        param_types.push(ty);
                    }
                }
                let ret_ty = self
                    .resolve_type_annotation(return_type)
                    .unwrap_or(NyType::Unit);
                let saved_ret = self.current_return_type.clone();
                self.current_return_type = ret_ty.clone();
                self.check_expr(body);
                self.current_return_type = saved_ret;
                self.pop_scope();
                NyType::Function {
                    params: param_types,
                    ret: Box::new(ret_ty),
                }
            }

            // ── Tuple index ──────────────────────────────────────────
            Expr::TupleIndex {
                object,
                index,
                span,
            } => {
                let obj_ty = self.check_expr(object);
                match &obj_ty {
                    NyType::Tuple(elems) => {
                        if *index >= elems.len() {
                            self.errors.push(CompileError::type_error(
                                format!(
                                    "tuple index {} out of range for tuple of length {}",
                                    index,
                                    elems.len()
                                ),
                                *span,
                            ));
                            NyType::I32
                        } else {
                            elems[*index].clone()
                        }
                    }
                    _ => {
                        self.errors.push(CompileError::type_error(
                            format!("cannot index into non-tuple type '{}'", obj_ty),
                            *span,
                        ));
                        NyType::I32
                    }
                }
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
                    let candidates: Vec<&str> = fields.iter().map(|(n, _)| n.as_str()).collect();
                    let mut err = CompileError::type_error(
                        format!("struct '{}' has no field '{}'", name, field),
                        span,
                    );
                    if let Some(suggestion) = Self::suggest_similar(field, &candidates) {
                        err = err.with_note(format!("did you mean '{}'?", suggestion));
                    }
                    self.errors.push(err);
                    NyType::I32
                }
            }
            // Auto-deref: if it's a pointer to a struct, dereference and access
            NyType::Pointer(inner) => self.resolve_field_access(inner, field, span),
            // Optional: prevent direct field access — must unwrap first
            NyType::Optional(inner) => {
                self.errors.push(CompileError::type_error(
                    format!(
                        "cannot access field '{}' on optional type '?{}' — unwrap with 'if let' or '??'",
                        field, inner
                    ),
                    span,
                ));
                NyType::I32
            }
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
        // chan<T> method calls
        if let NyType::Chan(elem_ty) = receiver_ty {
            for arg in args {
                self.check_expr(arg);
            }
            match method {
                "send" => return NyType::Unit,
                "recv" => return *elem_ty.clone(),
                "close" => return NyType::Unit,
                _ => {
                    self.errors.push(CompileError::type_error(
                        format!("chan has no method '{}'", method),
                        span,
                    ));
                    return NyType::Unit;
                }
            }
        }

        // dyn Trait method calls — look up return type from trait definition
        if let NyType::DynTrait(trait_name) = receiver_ty {
            for arg in args {
                self.check_expr(arg);
            }
            if let Some((sigs, _)) = self.traits.get(trait_name) {
                if let Some((_, _, ret_ty)) = sigs.iter().find(|(n, _, _)| n == method) {
                    return ret_ty.clone();
                }
            }
            self.errors.push(CompileError::type_error(
                format!("trait '{}' has no method '{}'", trait_name, method),
                span,
            ));
            return NyType::I32;
        }

        // Built-in Vec methods
        // HashMap<K,V> methods
        if let NyType::HashMap(_key_ty, val_ty) = receiver_ty {
            match method {
                "insert" => {
                    for arg in args {
                        self.check_expr(arg);
                    }
                    return NyType::Unit;
                }
                "get" => {
                    for arg in args {
                        self.check_expr(arg);
                    }
                    return *val_ty.clone();
                }
                "contains" => {
                    for arg in args {
                        self.check_expr(arg);
                    }
                    return NyType::Bool;
                }
                "len" => return NyType::I64,
                "remove" | "free" => {
                    for arg in args {
                        self.check_expr(arg);
                    }
                    return NyType::Unit;
                }
                _ => {
                    for arg in args {
                        self.check_expr(arg);
                    }
                    self.errors.push(CompileError::type_error(
                        format!("no method '{}' found for HashMap type", method),
                        span,
                    ));
                    return NyType::I32;
                }
            }
        }

        if let NyType::Vec(elem) = receiver_ty {
            match method {
                "push" => {
                    if args.len() == 1 {
                        let arg_ty = self.check_expr(&args[0]);
                        if arg_ty != **elem {
                            self.errors.push(CompileError::type_error(
                                format!("Vec push: expected '{}', found '{}'", elem, arg_ty),
                                args[0].span(),
                            ));
                        }
                    }
                    return NyType::Unit;
                }
                "len" => return NyType::I64,
                "get" => {
                    if args.len() == 1 {
                        self.check_expr(&args[0]);
                    }
                    return *elem.clone();
                }
                "set" => {
                    if args.len() == 2 {
                        self.check_expr(&args[0]); // index
                        let arg_ty = self.check_expr(&args[1]);
                        if arg_ty != **elem {
                            self.errors.push(CompileError::type_error(
                                format!("Vec set: expected '{}', found '{}'", elem, arg_ty),
                                args[1].span(),
                            ));
                        }
                    }
                    return NyType::Unit;
                }
                "pop" | "sum" => return *elem.clone(),
                "join" => {
                    if args.len() == 1 {
                        self.check_expr(&args[0]);
                    }
                    return NyType::Str;
                }
                "sort" | "reverse" | "clear" => return NyType::Unit,
                "map" | "filter" => {
                    if args.len() == 1 {
                        self.check_expr(&args[0]);
                    }
                    return NyType::Vec(elem.clone());
                }
                "reduce" => {
                    for arg in args {
                        self.check_expr(arg);
                    }
                    return *elem.clone();
                }
                "for_each" => {
                    if args.len() == 1 {
                        self.check_expr(&args[0]);
                    }
                    return NyType::Unit;
                }
                "any" | "all" => {
                    if args.len() == 1 {
                        self.check_expr(&args[0]);
                    }
                    return NyType::Bool;
                }
                "contains" => {
                    if args.len() == 1 {
                        self.check_expr(&args[0]);
                    }
                    return NyType::Bool;
                }
                "index_of" => {
                    if args.len() == 1 {
                        self.check_expr(&args[0]);
                    }
                    return NyType::I32;
                }
                _ => {
                    for arg in args {
                        self.check_expr(arg);
                    }
                    let vec_methods = &[
                        "push", "pop", "get", "set", "len", "sort", "reverse", "clear", "contains",
                        "index_of", "map", "filter", "reduce", "for_each", "any", "all",
                    ];
                    let mut err = CompileError::type_error(
                        format!("no method '{}' found for Vec type", method),
                        span,
                    );
                    if let Some(s) = Self::suggest_similar(method, vec_methods) {
                        err = err.with_note(format!("did you mean '{}'?", s));
                    }
                    self.errors.push(err);
                    return NyType::I32;
                }
            }
        }

        // Built-in slice methods
        if let NyType::Slice(_) = receiver_ty {
            match method {
                "len" => {
                    if !args.is_empty() {
                        self.errors.push(CompileError::type_error(
                            format!("'len' takes no arguments, found {}", args.len()),
                            span,
                        ));
                    }
                    return NyType::I64;
                }
                _ => {
                    for arg in args {
                        self.check_expr(arg);
                    }
                    self.errors.push(CompileError::type_error(
                        format!("no method '{}' found for slice type", method),
                        span,
                    ));
                    return NyType::I32;
                }
            }
        }

        // Built-in string methods
        if *receiver_ty == NyType::Str {
            match method {
                "len" => {
                    if !args.is_empty() {
                        self.errors.push(CompileError::type_error(
                            format!("'len' takes no arguments, found {}", args.len()),
                            span,
                        ));
                    }
                    return NyType::I64;
                }
                "substr" => {
                    if args.len() != 2 {
                        self.errors.push(CompileError::type_error(
                            format!("'substr' expects 2 arguments, found {}", args.len()),
                            span,
                        ));
                    } else {
                        let arg0_ty = self.check_expr(&args[0]);
                        let arg1_ty = self.check_expr(&args[1]);
                        if arg0_ty != NyType::I64 {
                            self.errors.push(CompileError::type_error(
                                format!(
                                    "argument 1 of 'substr': expected 'i64', found '{}'",
                                    arg0_ty
                                ),
                                args[0].span(),
                            ));
                        }
                        if arg1_ty != NyType::I64 {
                            self.errors.push(CompileError::type_error(
                                format!(
                                    "argument 2 of 'substr': expected 'i64', found '{}'",
                                    arg1_ty
                                ),
                                args[1].span(),
                            ));
                        }
                    }
                    return NyType::Str;
                }
                "trim" => {
                    return NyType::Str;
                }
                "to_upper" | "to_lower" => {
                    return NyType::Str;
                }
                "repeat" => {
                    if args.len() == 1 {
                        self.check_expr(&args[0]);
                    }
                    return NyType::Str;
                }
                "replace" => {
                    if args.len() == 2 {
                        self.check_expr(&args[0]);
                        self.check_expr(&args[1]);
                    }
                    return NyType::Str;
                }
                "char_at" => {
                    if args.len() == 1 {
                        self.check_expr(&args[0]);
                    }
                    return NyType::I32;
                }
                "contains" | "starts_with" | "ends_with" => {
                    if args.len() == 1 {
                        let arg_ty = self.check_expr(&args[0]);
                        if arg_ty != NyType::Str {
                            self.errors.push(CompileError::type_error(
                                format!("'{}' expects str argument, found '{}'", method, arg_ty),
                                args[0].span(),
                            ));
                        }
                    }
                    return NyType::Bool;
                }
                "index_of" => {
                    if args.len() == 1 {
                        let arg_ty = self.check_expr(&args[0]);
                        if arg_ty != NyType::Str {
                            self.errors.push(CompileError::type_error(
                                format!("'index_of' expects str argument, found '{}'", arg_ty),
                                args[0].span(),
                            ));
                        }
                    }
                    return NyType::I32;
                }
                _ => {
                    for arg in args {
                        self.check_expr(arg);
                    }
                    let str_methods = &[
                        "len",
                        "substr",
                        "char_at",
                        "contains",
                        "starts_with",
                        "ends_with",
                        "index_of",
                        "trim",
                        "to_upper",
                        "to_lower",
                        "replace",
                        "repeat",
                    ];
                    let mut err = CompileError::type_error(
                        format!("no method '{}' found for type 'str'", method),
                        span,
                    );
                    if let Some(s) = Self::suggest_similar(method, str_methods) {
                        err = err.with_note(format!("did you mean '{}'?", s));
                    }
                    self.errors.push(err);
                    return NyType::I32;
                }
            }
        }

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
                        // Allow numeric coercion, Vec coercion, chan coercion, dyn Trait, and optional coercion
                        let optional_compat = matches!(&declared_ty, NyType::Optional(_));
                        let compatible = init_ty == declared_ty
                            || (init_ty.is_vec() && declared_ty.is_vec())
                            || (init_ty.is_hashmap() && declared_ty.is_hashmap())
                            || (init_ty.is_numeric() && declared_ty.is_numeric())
                            || matches!(&declared_ty, NyType::DynTrait(_))
                            || matches!(
                                (&init_ty, &declared_ty),
                                (NyType::Chan(_), NyType::Chan(_))
                            )
                            || optional_compat;
                        if !compatible {
                            self.errors.push(CompileError::type_error(
                                format!("expected '{}', found '{}'", declared_ty, init_ty),
                                init.span(),
                            ));
                        }
                        self.declare(name, declared_ty);
                    } else {
                        self.errors.push(CompileError::type_error(
                            "unknown type in annotation".to_string(),
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

                let ret_compatible = ret_ty == self.current_return_type
                    || (ret_ty.is_numeric() && self.current_return_type.is_numeric())
                    || matches!(&self.current_return_type, NyType::DynTrait(_))
                    || matches!(&self.current_return_type, NyType::Optional(_));
                if !ret_compatible {
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

            // ── ForIn ─────────────────────────────────────────────────
            Stmt::ForIn {
                var,
                collection,
                body,
                ..
            } => {
                let coll_ty = self.check_expr(collection);
                let elem_ty = match &coll_ty {
                    NyType::Array { elem, .. } => *elem.clone(),
                    NyType::Slice(elem) => *elem.clone(),
                    NyType::Vec(elem) => *elem.clone(),
                    _ => {
                        self.errors.push(CompileError::type_error(
                            format!("cannot iterate over type '{}'", coll_ty),
                            collection.span(),
                        ));
                        NyType::I32
                    }
                };
                self.push_scope();
                self.declare(var, elem_ty);
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

            // ── WhileLet ──────────────────────────────────────────────
            Stmt::WhileLet {
                pattern,
                expr,
                body,
                ..
            } => {
                let expr_ty = self.check_expr(expr);
                if let Pattern::EnumVariant {
                    variant, bindings, ..
                } = pattern
                {
                    if let NyType::Enum { variants, .. } = &expr_ty {
                        if let Some((_, payload)) = variants.iter().find(|(n, _)| n == variant) {
                            if !bindings.is_empty() {
                                self.push_scope();
                                for (i, b) in bindings.iter().enumerate() {
                                    let ty = payload.get(i).cloned().unwrap_or(NyType::I32);
                                    self.declare(b, ty);
                                }
                                self.loop_depth += 1;
                                self.check_expr(body);
                                self.loop_depth -= 1;
                                self.pop_scope();
                                return;
                            }
                        }
                    }
                }
                self.loop_depth += 1;
                self.check_expr(body);
                self.loop_depth -= 1;
            }

            // ── IfLet ─────────────────────────────────────────────────
            Stmt::IfLet {
                pattern,
                expr,
                then_body,
                else_body,
                ..
            } => {
                let expr_ty = self.check_expr(expr);
                // Declare bindings in then_body scope
                if let Pattern::EnumVariant {
                    enum_name,
                    variant,
                    bindings,
                    ..
                } = pattern
                {
                    if let NyType::Enum { variants, .. } = &expr_ty {
                        if let Some((_, payload)) = variants.iter().find(|(n, _)| n == variant) {
                            if !bindings.is_empty() {
                                self.push_scope();
                                for (i, binding) in bindings.iter().enumerate() {
                                    let ty = payload.get(i).cloned().unwrap_or(NyType::I32);
                                    self.declare(binding, ty);
                                }
                                self.check_expr(then_body);
                                self.pop_scope();
                            } else {
                                self.check_expr(then_body);
                            }
                        } else {
                            self.errors.push(CompileError::type_error(
                                format!("enum '{}' has no variant '{}'", enum_name, variant),
                                expr.span(),
                            ));
                            self.check_expr(then_body);
                        }
                    }
                } else if let Pattern::OptionalBind { name, .. } = pattern {
                    // ?T unwrap: bind the inner type
                    let inner = match &expr_ty {
                        NyType::Optional(inner) => *inner.clone(),
                        NyType::Pointer(_) => expr_ty.clone(),
                        _ => expr_ty.clone(),
                    };
                    self.push_scope();
                    self.declare(name, inner);
                    self.check_expr(then_body);
                    self.pop_scope();
                } else {
                    self.check_expr(then_body);
                }
                if let Some(eb) = else_body {
                    self.check_expr(eb);
                }
            }

            // ── Defer ─────────────────────────────────────────────────
            Stmt::Defer { body, .. } => {
                self.check_expr(body);
            }

            // ── Loop ─────────────────────────────────────────────────
            Stmt::Loop { body, .. } => {
                self.loop_depth += 1;
                self.check_expr(body);
                self.loop_depth -= 1;
            }

            Stmt::ForMap {
                key_var,
                val_var,
                map_expr,
                body,
                ..
            } => {
                let map_ty = self.check_expr(map_expr);
                let (key_ty, val_ty) = match &map_ty {
                    NyType::HashMap(k, v) => (*k.clone(), *v.clone()),
                    _ => (NyType::Str, NyType::I32),
                };
                self.push_scope();
                self.declare(key_var, key_ty);
                self.declare(val_var, val_ty);
                self.check_expr(body);
                self.pop_scope();
            }

            // ── Select ──────────────────────────────────────────────
            Stmt::Select { arms, .. } => {
                for arm in arms {
                    let ch_ty = self.check_expr(&arm.channel);
                    // Determine recv type from channel method call
                    let var_ty = if let Expr::MethodCall { object, .. } = &arm.channel {
                        let obj_ty = self.check_expr(object);
                        match obj_ty {
                            NyType::Chan(elem) => *elem,
                            _ => NyType::I32,
                        }
                    } else {
                        match ch_ty {
                            NyType::I32 => NyType::I32,
                            _ => NyType::I32,
                        }
                    };
                    self.push_scope();
                    self.declare(&arm.var, var_ty);
                    self.check_expr(&arm.body);
                    self.pop_scope();
                }
            }

            // ── Tuple destructure ────────────────────────────────────
            Stmt::TupleDestructure {
                names, init, span, ..
            } => {
                let init_ty = self.check_expr(init);
                match &init_ty {
                    NyType::Tuple(elems) => {
                        if names.len() != elems.len() {
                            self.errors.push(CompileError::type_error(
                                format!(
                                    "tuple destructure: expected {} names for tuple of length {}, found {}",
                                    elems.len(),
                                    elems.len(),
                                    names.len()
                                ),
                                *span,
                            ));
                            // Declare names with Unit as fallback
                            for name in names {
                                self.declare(name, NyType::Unit);
                            }
                        } else {
                            for (name, elem_ty) in names.iter().zip(elems.iter()) {
                                self.declare(name, elem_ty.clone());
                            }
                        }
                    }
                    _ => {
                        self.errors.push(CompileError::type_error(
                            format!(
                                "tuple destructure requires a tuple type, found '{}'",
                                init_ty
                            ),
                            *span,
                        ));
                        for name in names {
                            self.declare(name, NyType::Unit);
                        }
                    }
                }
            }
        }
    }
}
