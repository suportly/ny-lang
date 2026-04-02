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
    // Step 0: Monomorphize generic structs and enums
    monomorphize_structs(program);

    // Step 0b: Rewrite generic enum variant references
    // e.g., Option::Some(42) → Option_i32::Some(42)
    rewrite_generic_enum_variants(program);

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
            Expr::Call { callee: _, .. } => {
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

// ---------------------------------------------------------------------------
// Generic struct monomorphization
// ---------------------------------------------------------------------------

fn monomorphize_structs(program: &mut Program) {
    let mut generic_structs: HashMap<String, Item> = HashMap::new();
    for item in &program.items {
        match item {
            Item::StructDef {
                name, type_params, ..
            } if !type_params.is_empty() => {
                generic_structs.insert(name.clone(), item.clone());
            }
            Item::EnumDef {
                name, type_params, ..
            } if !type_params.is_empty() => {
                generic_structs.insert(name.clone(), item.clone());
            }
            _ => {}
        }
    }

    if generic_structs.is_empty() {
        return;
    }

    // Scan type annotations for usages like "Pair<i32,bool>"
    let mut struct_usages: Vec<(String, Vec<String>)> = Vec::new();
    for item in &program.items {
        collect_generic_type_usages_in_item(item, &generic_structs, &mut struct_usages);
    }
    struct_usages.sort();
    struct_usages.dedup();

    // Generate concrete struct definitions
    let mut new_structs: Vec<Item> = Vec::new();
    for (base_name, type_args) in &struct_usages {
        if let Some(template) = generic_structs.get(base_name) {
            match template {
                Item::StructDef { .. } => {
                    if let Some(concrete) = specialize_struct(template, type_args) {
                        new_structs.push(concrete);
                    }
                }
                Item::EnumDef { .. } => {
                    if let Some(concrete) = specialize_enum(template, type_args) {
                        new_structs.push(concrete);
                    }
                }
                _ => {}
            }
        }
    }

    // Build rewrite map: "Pair<i32,bool>" → "Pair_i32_bool"
    let rewrite_map: HashMap<String, String> = struct_usages
        .iter()
        .map(|(base, args)| {
            let key = format!("{}<{}>", base, args.join(","));
            let val = format!("{}_{}", base, args.join("_"));
            (key, val)
        })
        .collect();

    for item in &mut program.items {
        rewrite_type_names_in_item(item, &rewrite_map);
    }

    // Remove generic templates, add concrete versions
    program.items.retain(|item| {
        !matches!(item, Item::StructDef { type_params, .. } if !type_params.is_empty())
            && !matches!(item, Item::EnumDef { type_params, .. } if !type_params.is_empty())
    });
    program.items.splice(0..0, new_structs);
}

fn specialize_enum(template: &Item, type_args: &[String]) -> Option<Item> {
    if let Item::EnumDef {
        name,
        type_params,
        variants,
        span,
    } = template
    {
        let type_map: HashMap<String, String> = type_params
            .iter()
            .zip(type_args.iter())
            .map(|(tp, arg)| (tp.clone(), arg.clone()))
            .collect();

        let mangled_name = format!("{}_{}", name, type_args.join("_"));
        let new_variants: Vec<EnumVariantDef> = variants
            .iter()
            .map(|v| EnumVariantDef {
                name: v.name.clone(),
                payload: v
                    .payload
                    .iter()
                    .map(|ty| substitute_type_str(ty, &type_map))
                    .collect(),
                span: v.span,
            })
            .collect();

        Some(Item::EnumDef {
            name: mangled_name,
            type_params: Vec::new(),
            variants: new_variants,
            span: *span,
        })
    } else {
        None
    }
}

fn specialize_struct(template: &Item, type_args: &[String]) -> Option<Item> {
    if let Item::StructDef {
        name,
        type_params,
        fields,
        span,
    } = template
    {
        let type_map: HashMap<String, String> = type_params
            .iter()
            .zip(type_args.iter())
            .map(|(tp, arg)| (tp.clone(), arg.clone()))
            .collect();

        let mangled_name = format!("{}_{}", name, type_args.join("_"));
        let new_fields: Vec<(String, TypeAnnotation)> = fields
            .iter()
            .map(|(fname, fty)| (fname.clone(), substitute_type_str(fty, &type_map)))
            .collect();

        Some(Item::StructDef {
            name: mangled_name,
            type_params: Vec::new(),
            fields: new_fields,
            span: *span,
        })
    } else {
        None
    }
}

fn substitute_type_str(ann: &TypeAnnotation, map: &HashMap<String, String>) -> TypeAnnotation {
    match ann {
        TypeAnnotation::Named { name, span } => {
            if let Some(concrete) = map.get(name) {
                TypeAnnotation::Named {
                    name: concrete.clone(),
                    span: *span,
                }
            } else {
                ann.clone()
            }
        }
        _ => ann.clone(),
    }
}

fn collect_generic_type_usages_in_item(
    item: &Item,
    generic_structs: &HashMap<String, Item>,
    usages: &mut Vec<(String, Vec<String>)>,
) {
    match item {
        Item::FunctionDef {
            params,
            return_type,
            ..
        } => {
            for p in params {
                collect_generic_in_annotation(&p.ty, generic_structs, usages);
            }
            collect_generic_in_annotation(return_type, generic_structs, usages);
        }
        Item::ImplBlock { methods, .. } => {
            for m in methods {
                collect_generic_type_usages_in_item(m, generic_structs, usages);
            }
        }
        _ => {}
    }
}

fn collect_generic_in_annotation(
    ann: &TypeAnnotation,
    generic_structs: &HashMap<String, Item>,
    usages: &mut Vec<(String, Vec<String>)>,
) {
    if let TypeAnnotation::Named { name, .. } = ann {
        if let Some(open) = name.find('<') {
            let base = &name[..open];
            if generic_structs.contains_key(base) {
                let args_str = &name[open + 1..name.len() - 1];
                let args: Vec<String> =
                    args_str.split(',').map(|s| s.trim().to_string()).collect();
                usages.push((base.to_string(), args));
            }
        }
    }
}

fn rewrite_type_names_in_item(item: &mut Item, map: &HashMap<String, String>) {
    match item {
        Item::FunctionDef {
            params,
            return_type,
            ..
        } => {
            for p in params {
                rewrite_annotation(&mut p.ty, map);
            }
            rewrite_annotation(return_type, map);
        }
        Item::ImplBlock { methods, .. } => {
            for m in methods {
                rewrite_type_names_in_item(m, map);
            }
        }
        _ => {}
    }
}

fn rewrite_annotation(ann: &mut TypeAnnotation, map: &HashMap<String, String>) {
    if let TypeAnnotation::Named { name, .. } = ann {
        if let Some(replacement) = map.get(name.as_str()) {
            *name = replacement.clone();
        }
    }
}

// ---------------------------------------------------------------------------
// Rewrite generic enum variant references: Option::Some(42) → Option_i32::Some(42)
// ---------------------------------------------------------------------------

fn rewrite_generic_enum_variants(program: &mut Program) {
    // Collect concrete enum names: "Option_i32", "Result_i32_i32", etc.
    // Build map: base_name → [(mangled_name, variant_names)]
    let mut enum_map: HashMap<String, Vec<String>> = HashMap::new();
    for item in &program.items {
        if let Item::EnumDef {
            name, type_params, ..
        } = item
        {
            if type_params.is_empty() {
                // This is a concrete (possibly monomorphized) enum
                // Extract base name: "Option_i32" → "Option"
                if let Some(underscore_pos) = name.find('_') {
                    let base = &name[..underscore_pos];
                    enum_map
                        .entry(base.to_string())
                        .or_default()
                        .push(name.clone());
                }
            }
        }
    }

    if enum_map.is_empty() {
        return;
    }

    // Rewrite EnumVariant expressions and patterns in all items
    for item in &mut program.items {
        rewrite_enum_refs_in_item(item, &enum_map);
    }
}

fn rewrite_enum_refs_in_item(item: &mut Item, map: &HashMap<String, Vec<String>>) {
    match item {
        Item::FunctionDef { body, .. } => {
            rewrite_enum_refs_in_expr(body, map);
        }
        Item::ImplBlock { methods, .. } => {
            for m in methods {
                rewrite_enum_refs_in_item(m, map);
            }
        }
        _ => {}
    }
}

fn rewrite_enum_refs_in_expr(expr: &mut Expr, map: &HashMap<String, Vec<String>>) {
    match expr {
        Expr::EnumVariant { enum_name, .. } => {
            // If enum_name is a generic base like "Option", replace with first concrete version
            if let Some(concretes) = map.get(enum_name.as_str()) {
                if let Some(first) = concretes.first() {
                    *enum_name = first.clone();
                }
            }
        }
        Expr::Match { subject, arms, .. } => {
            rewrite_enum_refs_in_expr(subject, map);
            for arm in arms {
                // Rewrite pattern enum names
                if let Pattern::EnumVariant { enum_name, .. } = &mut arm.pattern {
                    if let Some(concretes) = map.get(enum_name.as_str()) {
                        if let Some(first) = concretes.first() {
                            *enum_name = first.clone();
                        }
                    }
                }
                rewrite_enum_refs_in_expr(&mut arm.body, map);
            }
        }
        Expr::Block { stmts, tail_expr, .. } => {
            for stmt in stmts {
                rewrite_enum_refs_in_stmt(stmt, map);
            }
            if let Some(te) = tail_expr {
                rewrite_enum_refs_in_expr(te, map);
            }
        }
        Expr::If { condition, then_branch, else_branch, .. } => {
            rewrite_enum_refs_in_expr(condition, map);
            rewrite_enum_refs_in_expr(then_branch, map);
            if let Some(eb) = else_branch {
                rewrite_enum_refs_in_expr(eb, map);
            }
        }
        Expr::BinOp { lhs, rhs, .. } => {
            rewrite_enum_refs_in_expr(lhs, map);
            rewrite_enum_refs_in_expr(rhs, map);
        }
        Expr::UnaryOp { operand, .. } => {
            rewrite_enum_refs_in_expr(operand, map);
        }
        Expr::Call { args, .. } => {
            for arg in args {
                rewrite_enum_refs_in_expr(arg, map);
            }
        }
        Expr::Try { operand, .. } => {
            rewrite_enum_refs_in_expr(operand, map);
        }
        _ => {}
    }
}

fn rewrite_enum_refs_in_stmt(stmt: &mut Stmt, map: &HashMap<String, Vec<String>>) {
    match stmt {
        Stmt::VarDecl { init, .. } => rewrite_enum_refs_in_expr(init, map),
        Stmt::ConstDecl { value, .. } => rewrite_enum_refs_in_expr(value, map),
        Stmt::ExprStmt { expr, .. } => rewrite_enum_refs_in_expr(expr, map),
        Stmt::Return { value, .. } => {
            if let Some(v) = value {
                rewrite_enum_refs_in_expr(v, map);
            }
        }
        Stmt::Assign { value, .. } => rewrite_enum_refs_in_expr(value, map),
        Stmt::While { condition, body, .. } => {
            rewrite_enum_refs_in_expr(condition, map);
            rewrite_enum_refs_in_expr(body, map);
        }
        Stmt::ForRange { start, end, body, .. } => {
            rewrite_enum_refs_in_expr(start, map);
            rewrite_enum_refs_in_expr(end, map);
            rewrite_enum_refs_in_expr(body, map);
        }
        Stmt::Loop { body, .. } | Stmt::Defer { body, .. } => {
            rewrite_enum_refs_in_expr(body, map);
        }
        Stmt::IfLet { expr, then_body, else_body, pattern, .. } => {
            rewrite_enum_refs_in_expr(expr, map);
            if let Pattern::EnumVariant { enum_name, .. } = pattern {
                if let Some(concretes) = map.get(enum_name.as_str()) {
                    if let Some(first) = concretes.first() {
                        *enum_name = first.clone();
                    }
                }
            }
            rewrite_enum_refs_in_expr(then_body, map);
            if let Some(eb) = else_body {
                rewrite_enum_refs_in_expr(eb, map);
            }
        }
        Stmt::WhileLet { expr, body, pattern, .. } => {
            rewrite_enum_refs_in_expr(expr, map);
            if let Pattern::EnumVariant { enum_name, .. } = pattern {
                if let Some(concretes) = map.get(enum_name.as_str()) {
                    if let Some(first) = concretes.first() {
                        *enum_name = first.clone();
                    }
                }
            }
            rewrite_enum_refs_in_expr(body, map);
        }
        _ => {}
    }
}
