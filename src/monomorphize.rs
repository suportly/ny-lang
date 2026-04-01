//! Monomorphization pass: specializes generic functions for concrete types.
//!
//! When we see `fn max<T>(a: T, b: T) -> T { ... }` called as `max(1, 2)`,
//! we generate `fn max_i32(a: i32, b: i32) -> i32 { ... }` and rewrite the call.

use std::collections::HashMap;

use crate::common::NyType;
use crate::parser::ast::*;

/// Run monomorphization on the program AST.
/// - Finds generic functions (type_params non-empty)
/// - Finds calls to them
/// - Infers concrete types from call-site args (using simple literal type inference)
/// - Generates specialized copies
/// - Rewrites calls to point to specialized versions
pub fn monomorphize(program: &mut Program) {
    // Step 1: Collect generic function templates
    let mut generic_fns: HashMap<String, Item> = HashMap::new();
    for item in &program.items {
        if let Item::FunctionDef {
            name, type_params, ..
        } = item
        {
            if !type_params.is_empty() {
                generic_fns.insert(name.clone(), item.clone());
            }
        }
    }

    if generic_fns.is_empty() {
        return; // Nothing to monomorphize
    }

    // Step 2: Scan for calls to generic functions and collect specializations needed
    let mut specializations: Vec<(String, Vec<NyType>)> = Vec::new();
    for item in &program.items {
        collect_specializations_in_item(item, &generic_fns, &mut specializations);
    }

    // Deduplicate
    specializations.sort_by(|a, b| a.0.cmp(&b.0));
    specializations.dedup();

    // Step 3: Generate monomorphized copies
    let mut new_items: Vec<Item> = Vec::new();
    for (fn_name, concrete_types) in &specializations {
        if let Some(template) = generic_fns.get(fn_name) {
            if let Some(specialized) = specialize_function(template, concrete_types) {
                new_items.push(specialized);
            }
        }
    }

    // Step 4: Rewrite calls in all items
    let specs_map: HashMap<String, Vec<(Vec<NyType>, String)>> = {
        let mut map: HashMap<String, Vec<(Vec<NyType>, String)>> = HashMap::new();
        for (fn_name, concrete_types) in &specializations {
            let mangled = mangle_name(fn_name, concrete_types);
            map.entry(fn_name.clone())
                .or_default()
                .push((concrete_types.clone(), mangled));
        }
        map
    };

    for item in &mut program.items {
        rewrite_calls_in_item(item, &specs_map);
    }

    // Step 5: Remove generic function templates and add specialized ones
    program
        .items
        .retain(|item| !matches!(item, Item::FunctionDef { type_params, .. } if !type_params.is_empty()));
    program.items.extend(new_items);
}

fn mangle_name(name: &str, types: &[NyType]) -> String {
    let type_suffix: Vec<String> = types.iter().map(|t| format!("{}", t)).collect();
    format!("{}_{}", name, type_suffix.join("_"))
}

/// Simple type inference for monomorphization.
/// Tracks variable types from VarDecl and literal types.
struct SimpleTypeEnv {
    vars: HashMap<String, NyType>,
}

impl SimpleTypeEnv {
    fn new() -> Self {
        Self {
            vars: HashMap::new(),
        }
    }

    fn infer(&self, expr: &Expr) -> Option<NyType> {
        match expr {
            Expr::Literal { value, .. } => match value {
                LitValue::Int(_) => Some(NyType::I32),
                LitValue::Float(_) => Some(NyType::F64),
                LitValue::Bool(_) => Some(NyType::Bool),
                LitValue::Str(_) => Some(NyType::Str),
            },
            Expr::Ident { name, .. } => self.vars.get(name).cloned(),
            Expr::Call { callee, .. } => {
                // If calling a known non-generic function, we'd need its return type
                // For now, fall back to None
                None
            }
            Expr::BinOp { lhs, .. } => self.infer(lhs),
            Expr::UnaryOp { operand, .. } => self.infer(operand),
            _ => None,
        }
    }

    fn record_var(&mut self, name: &str, init: &Expr) {
        if let Some(ty) = self.infer(init) {
            self.vars.insert(name.to_string(), ty);
        }
    }

    /// Record a variable from a call to a generic function that we've already specialized.
    fn record_var_from_call(&mut self, name: &str, callee: &str, generic_fns: &HashMap<String, Item>, concrete_types: &[NyType]) {
        if let Some(template) = generic_fns.get(callee) {
            if let Item::FunctionDef { type_params, return_type, .. } = template {
                let type_map: HashMap<String, NyType> = type_params.iter()
                    .zip(concrete_types.iter())
                    .map(|(tp, ct)| (tp.clone(), ct.clone()))
                    .collect();
                if let TypeAnnotation::Named { name: ret_name, .. } = return_type {
                    if let Some(concrete) = type_map.get(ret_name) {
                        self.vars.insert(name.to_string(), concrete.clone());
                    }
                }
            }
        }
    }
}

fn collect_specializations_in_item(
    item: &Item,
    generic_fns: &HashMap<String, Item>,
    specs: &mut Vec<(String, Vec<NyType>)>,
) {
    let mut env = SimpleTypeEnv::new();
    match item {
        Item::FunctionDef { params, body, .. } => {
            // Seed env with function parameter types
            for p in params {
                if let TypeAnnotation::Named { name, .. } = &p.ty {
                    if let Some(ty) = NyType::from_name(name) {
                        env.vars.insert(p.name.clone(), ty);
                    }
                }
            }
            collect_specializations_in_expr(body, generic_fns, specs, &mut env);
        }
        Item::ImplBlock { methods, .. } => {
            for method in methods {
                collect_specializations_in_item(method, generic_fns, specs);
            }
        }
        _ => {}
    }
}

fn collect_specializations_in_expr(
    expr: &Expr,
    generic_fns: &HashMap<String, Item>,
    specs: &mut Vec<(String, Vec<NyType>)>,
    env: &mut SimpleTypeEnv,
) {
    match expr {
        Expr::Call { callee, args, .. } => {
            if let Some(template) = generic_fns.get(callee) {
                if let Item::FunctionDef {
                    type_params,
                    params,
                    ..
                } = template
                {
                    let mut type_map: HashMap<String, NyType> = HashMap::new();
                    for (i, arg) in args.iter().enumerate() {
                        if let Some(param) = params.get(i) {
                            if let TypeAnnotation::Named { name, .. } = &param.ty {
                                if type_params.contains(name) {
                                    if let Some(concrete) = env.infer(arg) {
                                        type_map.insert(name.clone(), concrete);
                                    }
                                }
                            }
                        }
                    }

                    if type_map.len() == type_params.len() {
                        let concrete_types: Vec<NyType> = type_params
                            .iter()
                            .map(|tp| type_map.get(tp).cloned().unwrap_or(NyType::I32))
                            .collect();
                        specs.push((callee.clone(), concrete_types));
                    }
                }
            }

            for arg in args {
                collect_specializations_in_expr(arg, generic_fns, specs, env);
            }
        }
        Expr::Block { stmts, tail_expr, .. } => {
            for stmt in stmts {
                collect_specializations_in_stmt(stmt, generic_fns, specs, env);
            }
            if let Some(te) = tail_expr {
                collect_specializations_in_expr(te, generic_fns, specs, env);
            }
        }
        Expr::If {
            condition,
            then_branch,
            else_branch,
            ..
        } => {
            collect_specializations_in_expr(condition, generic_fns, specs, env);
            collect_specializations_in_expr(then_branch, generic_fns, specs, env);
            if let Some(eb) = else_branch {
                collect_specializations_in_expr(eb, generic_fns, specs, env);
            }
        }
        Expr::BinOp { lhs, rhs, .. } => {
            collect_specializations_in_expr(lhs, generic_fns, specs, env);
            collect_specializations_in_expr(rhs, generic_fns, specs, env);
        }
        Expr::UnaryOp { operand, .. } => {
            collect_specializations_in_expr(operand, generic_fns, specs, env);
        }
        Expr::Match { subject, arms, .. } => {
            collect_specializations_in_expr(subject, generic_fns, specs, env);
            for arm in arms {
                collect_specializations_in_expr(&arm.body, generic_fns, specs, env);
            }
        }
        _ => {}
    }
}

fn collect_specializations_in_stmt(
    stmt: &Stmt,
    generic_fns: &HashMap<String, Item>,
    specs: &mut Vec<(String, Vec<NyType>)>,
    env: &mut SimpleTypeEnv,
) {
    match stmt {
        Stmt::VarDecl { name, init, .. } => {
            // Track the variable type for later inference
            if let Expr::Call { callee, args, .. } = init {
                if generic_fns.contains_key(callee.as_str()) {
                    // The variable's type comes from the generic return type
                    // Infer from the first arg
                    if let Some(arg) = args.first() {
                        if let Some(ty) = env.infer(arg) {
                            env.vars.insert(name.clone(), ty);
                        }
                    }
                }
            }
            env.record_var(name, init);
            collect_specializations_in_expr(init, generic_fns, specs, env);
        }
        Stmt::ConstDecl { value, .. } => {
            collect_specializations_in_expr(value, generic_fns, specs, env);
        }
        Stmt::ExprStmt { expr, .. } => {
            collect_specializations_in_expr(expr, generic_fns, specs, env);
        }
        Stmt::Return { value, .. } => {
            if let Some(v) = value {
                collect_specializations_in_expr(v, generic_fns, specs, env);
            }
        }
        Stmt::While { condition, body, .. } => {
            collect_specializations_in_expr(condition, generic_fns, specs, env);
            collect_specializations_in_expr(body, generic_fns, specs, env);
        }
        Stmt::ForRange {
            start, end, body, ..
        } => {
            collect_specializations_in_expr(start, generic_fns, specs, env);
            collect_specializations_in_expr(end, generic_fns, specs, env);
            collect_specializations_in_expr(body, generic_fns, specs, env);
        }
        Stmt::Assign { value, .. } => {
            collect_specializations_in_expr(value, generic_fns, specs, env);
        }
        Stmt::Defer { body, .. } => {
            collect_specializations_in_expr(body, generic_fns, specs, env);
        }
        Stmt::Loop { body, .. } => {
            collect_specializations_in_expr(body, generic_fns, specs, env);
        }
        _ => {}
    }
}

/// Create a specialized version of a generic function by substituting type params.
fn specialize_function(template: &Item, concrete_types: &[NyType]) -> Option<Item> {
    if let Item::FunctionDef {
        name,
        type_params,
        params,
        return_type,
        body,
        span,
    } = template
    {
        let type_map: HashMap<String, NyType> = type_params
            .iter()
            .zip(concrete_types.iter())
            .map(|(tp, ct)| (tp.clone(), ct.clone()))
            .collect();

        let mangled_name = mangle_name(name, concrete_types);
        let new_params: Vec<Param> = params
            .iter()
            .map(|p| Param {
                name: p.name.clone(),
                ty: substitute_type_annotation(&p.ty, &type_map),
                span: p.span,
            })
            .collect();
        let new_return_type = substitute_type_annotation(return_type, &type_map);

        Some(Item::FunctionDef {
            name: mangled_name,
            type_params: Vec::new(), // Specialized — no more type params
            params: new_params,
            return_type: new_return_type,
            body: body.clone(), // Body stays the same; type annotations in it are substituted
            span: *span,
        })
    } else {
        None
    }
}

fn substitute_type_annotation(
    ann: &TypeAnnotation,
    type_map: &HashMap<String, NyType>,
) -> TypeAnnotation {
    match ann {
        TypeAnnotation::Named { name, span } => {
            if let Some(concrete) = type_map.get(name) {
                // Replace type param with concrete type name
                TypeAnnotation::Named {
                    name: format!("{}", concrete),
                    span: *span,
                }
            } else {
                ann.clone()
            }
        }
        TypeAnnotation::Array { elem, size, span } => TypeAnnotation::Array {
            elem: Box::new(substitute_type_annotation(elem, type_map)),
            size: *size,
            span: *span,
        },
        TypeAnnotation::Pointer { inner, span } => TypeAnnotation::Pointer {
            inner: Box::new(substitute_type_annotation(inner, type_map)),
            span: *span,
        },
        TypeAnnotation::Tuple { elements, span } => TypeAnnotation::Tuple {
            elements: elements
                .iter()
                .map(|e| Box::new(substitute_type_annotation(e, type_map)))
                .collect(),
            span: *span,
        },
        _ => ann.clone(),
    }
}

fn rewrite_calls_in_item(item: &mut Item, specs: &HashMap<String, Vec<(Vec<NyType>, String)>>) {
    match item {
        Item::FunctionDef { body, .. } => {
            rewrite_calls_in_expr(body, specs);
        }
        Item::ImplBlock { methods, .. } => {
            for method in methods {
                rewrite_calls_in_item(method, specs);
            }
        }
        _ => {}
    }
}

fn rewrite_calls_in_expr(expr: &mut Expr, specs: &HashMap<String, Vec<(Vec<NyType>, String)>>) {
    match expr {
        Expr::Call { callee, args, .. } => {
            // Rewrite generic call to monomorphized version
            if let Some(versions) = specs.get(callee.as_str()) {
                // Infer which version based on arg types
                if let Some((_, mangled)) = versions.first() {
                    *callee = mangled.clone();
                }
            }
            for arg in args {
                rewrite_calls_in_expr(arg, specs);
            }
        }
        Expr::Block { stmts, tail_expr, .. } => {
            for stmt in stmts {
                rewrite_calls_in_stmt(stmt, specs);
            }
            if let Some(te) = tail_expr {
                rewrite_calls_in_expr(te, specs);
            }
        }
        Expr::If {
            condition,
            then_branch,
            else_branch,
            ..
        } => {
            rewrite_calls_in_expr(condition, specs);
            rewrite_calls_in_expr(then_branch, specs);
            if let Some(eb) = else_branch {
                rewrite_calls_in_expr(eb, specs);
            }
        }
        Expr::BinOp { lhs, rhs, .. } => {
            rewrite_calls_in_expr(lhs, specs);
            rewrite_calls_in_expr(rhs, specs);
        }
        Expr::UnaryOp { operand, .. } => {
            rewrite_calls_in_expr(operand, specs);
        }
        Expr::Match { subject, arms, .. } => {
            rewrite_calls_in_expr(subject, specs);
            for arm in arms {
                rewrite_calls_in_expr(&mut arm.body, specs);
            }
        }
        _ => {}
    }
}

fn rewrite_calls_in_stmt(stmt: &mut Stmt, specs: &HashMap<String, Vec<(Vec<NyType>, String)>>) {
    match stmt {
        Stmt::VarDecl { init, .. } => {
            rewrite_calls_in_expr(init, specs);
        }
        Stmt::ConstDecl { value, .. } => {
            rewrite_calls_in_expr(value, specs);
        }
        Stmt::ExprStmt { expr, .. } => {
            rewrite_calls_in_expr(expr, specs);
        }
        Stmt::Return { value, .. } => {
            if let Some(v) = value {
                rewrite_calls_in_expr(v, specs);
            }
        }
        Stmt::While { condition, body, .. } => {
            rewrite_calls_in_expr(condition, specs);
            rewrite_calls_in_expr(body, specs);
        }
        Stmt::ForRange {
            start, end, body, ..
        } => {
            rewrite_calls_in_expr(start, specs);
            rewrite_calls_in_expr(end, specs);
            rewrite_calls_in_expr(body, specs);
        }
        Stmt::Assign { value, .. } => {
            rewrite_calls_in_expr(value, specs);
        }
        Stmt::Defer { body, .. } => {
            rewrite_calls_in_expr(body, specs);
        }
        Stmt::Loop { body, .. } => {
            rewrite_calls_in_expr(body, specs);
        }
        _ => {}
    }
}
