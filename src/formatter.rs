//! Ny Lang source formatter.
//!
//! Opinionated, zero-config formatter: 4-space indentation, K&R braces.
//! Preserves comments by extracting them from the original source and
//! reinserting them based on source line positions.

use crate::common::Span;
use crate::parser::ast::*;

const INDENT: &str = "    ";

/// A comment extracted from source, with its line number and content.
struct Comment {
    line: usize,
    text: String,
    standalone: bool,
}

fn extract_comments(source: &str) -> Vec<Comment> {
    let mut comments = Vec::new();
    for (line_num, line) in source.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.starts_with("//") {
            comments.push(Comment {
                line: line_num,
                text: trimmed.to_string(),
                standalone: true,
            });
            continue;
        }
        if let Some(pos) = find_line_comment(line) {
            let comment_text = line[pos..].trim().to_string();
            if !comment_text.is_empty() {
                comments.push(Comment {
                    line: line_num,
                    text: comment_text,
                    standalone: false,
                });
            }
        }
    }
    comments
}

fn find_line_comment(line: &str) -> Option<usize> {
    let mut in_string = false;
    let mut prev = '\0';
    let bytes = line.as_bytes();
    for i in 0..bytes.len() {
        let ch = bytes[i] as char;
        if ch == '"' && prev != '\\' {
            in_string = !in_string;
        }
        if !in_string && ch == '/' && i + 1 < bytes.len() && bytes[i + 1] == b'/' {
            let before = line[..i].trim();
            if !before.is_empty() {
                return Some(i);
            }
        }
        prev = ch;
    }
    None
}

fn byte_offset_to_line(source: &str, offset: usize) -> usize {
    source[..offset.min(source.len())]
        .chars()
        .filter(|&c| c == '\n')
        .count()
}

struct Fmt<'a> {
    source: &'a str,
    comments: Vec<Comment>,
    next_comment: usize,
}

impl<'a> Fmt<'a> {
    fn new(source: &'a str) -> Self {
        Self {
            source,
            comments: extract_comments(source),
            next_comment: 0,
        }
    }

    fn emit_comments_before(&mut self, out: &mut String, target_line: usize, depth: usize) {
        while self.next_comment < self.comments.len() {
            let c = &self.comments[self.next_comment];
            if c.line < target_line {
                if c.standalone {
                    indent(out, depth);
                    out.push_str(&c.text);
                    out.push('\n');
                }
                // Skip non-standalone (inline) — they're handled by emit_trailing_comment
                self.next_comment += 1;
            } else {
                break;
            }
        }
    }

    /// If there's a trailing comment on `line`, append it to the output.
    /// Must be called AFTER emitting the statement but BEFORE the newline.
    fn emit_trailing_comment(&self, out: &mut String, line: usize) {
        for c in &self.comments {
            if c.line == line && !c.standalone {
                out.push_str(" ");
                out.push_str(&c.text);
                return;
            }
        }
    }

    fn line_of(&self, span: Span) -> usize {
        byte_offset_to_line(self.source, span.start)
    }

    fn has_source(&self) -> bool {
        !self.source.is_empty()
    }
}

pub fn format_program(program: &Program) -> String {
    format_program_with_source(program, "")
}

pub fn format_program_with_source(program: &Program, source: &str) -> String {
    let mut f = Fmt::new(source);
    let mut out = String::new();
    let mut first = true;

    for item in &program.items {
        if f.has_source() {
            f.emit_comments_before(&mut out, f.line_of(item_span(item)), 0);
        }
        if !first {
            out.push('\n');
        }
        format_item(&mut out, item, 0, &mut f);
        first = false;
    }

    // Trailing comments
    while f.next_comment < f.comments.len() {
        let c = &f.comments[f.next_comment];
        if c.standalone {
            out.push_str(&c.text);
            out.push('\n');
        }
        f.next_comment += 1;
    }

    out
}

fn item_span(item: &Item) -> Span {
    match item {
        Item::FunctionDef { span, .. }
        | Item::StructDef { span, .. }
        | Item::EnumDef { span, .. }
        | Item::Use { span, .. }
        | Item::ExternBlock { span, .. }
        | Item::ImplBlock { span, .. }
        | Item::TraitDef { span, .. }
        | Item::TypeAlias { span, .. } => *span,
    }
}

fn stmt_span(stmt: &Stmt) -> Span {
    match stmt {
        Stmt::VarDecl { span, .. }
        | Stmt::ConstDecl { span, .. }
        | Stmt::Assign { span, .. }
        | Stmt::ExprStmt { span, .. }
        | Stmt::Return { span, .. }
        | Stmt::While { span, .. }
        | Stmt::ForRange { span, .. }
        | Stmt::ForIn { span, .. }
        | Stmt::Break { span, .. }
        | Stmt::Continue { span, .. }
        | Stmt::TupleDestructure { span, .. }
        | Stmt::Defer { span, .. }
        | Stmt::WhileLet { span, .. }
        | Stmt::IfLet { span, .. }
        | Stmt::Loop { span, .. }
        | Stmt::ForMap { span, .. }
        | Stmt::Select { span, .. } => *span,
    }
}

fn format_item(out: &mut String, item: &Item, depth: usize, f: &mut Fmt) {
    match item {
        Item::Use { path, .. } => {
            indent(out, depth);
            out.push_str(&format!("use \"{}\";\n", path));
        }
        Item::ExternBlock { functions, .. } => {
            indent(out, depth);
            out.push_str("extern {\n");
            for func in functions {
                indent(out, depth + 1);
                out.push_str(&format!("fn {}(", func.name));
                format_params(out, &func.params);
                out.push(')');
                format_return_type(out, &func.return_type);
                out.push_str(";\n");
            }
            indent(out, depth);
            out.push_str("}\n");
        }
        Item::StructDef {
            name,
            type_params,
            fields,
            ..
        } => {
            indent(out, depth);
            out.push_str(&format!("struct {}", name));
            format_type_params(out, type_params);
            out.push_str(" {\n");
            for (fname, ftype) in fields {
                indent(out, depth + 1);
                out.push_str(&format!("{}: {},\n", fname, format_type_annotation(ftype)));
            }
            indent(out, depth);
            out.push_str("}\n");
        }
        Item::EnumDef {
            name,
            type_params,
            variants,
            ..
        } => {
            indent(out, depth);
            out.push_str(&format!("enum {}", name));
            format_type_params(out, type_params);
            out.push_str(" {\n");
            for variant in variants {
                indent(out, depth + 1);
                out.push_str(&variant.name);
                if !variant.payload.is_empty() {
                    out.push('(');
                    let types: Vec<String> =
                        variant.payload.iter().map(format_type_annotation).collect();
                    out.push_str(&types.join(", "));
                    out.push(')');
                }
                out.push_str(",\n");
            }
            indent(out, depth);
            out.push_str("}\n");
        }
        Item::TraitDef { name, methods, .. } => {
            indent(out, depth);
            out.push_str(&format!("trait {} {{\n", name));
            for sig in methods {
                indent(out, depth + 1);
                out.push_str(&format!("fn {}(", sig.name));
                format_params(out, &sig.params);
                out.push(')');
                format_return_type(out, &sig.return_type);
                out.push_str(";\n");
            }
            indent(out, depth);
            out.push_str("}\n");
        }
        Item::TypeAlias { name, target, .. } => {
            indent(out, depth);
            out.push_str(&format!("type {} = {};\n", name, format_type_annotation(target)));
        }
        Item::ImplBlock {
            type_name,
            trait_name,
            methods,
            ..
        } => {
            indent(out, depth);
            if let Some(tname) = trait_name {
                out.push_str(&format!("impl {} for {} {{\n", tname, type_name));
            } else {
                out.push_str(&format!("impl {} {{\n", type_name));
            }
            for (i, method) in methods.iter().enumerate() {
                if i > 0 {
                    out.push('\n');
                }
                format_item(out, method, depth + 1, f);
            }
            indent(out, depth);
            out.push_str("}\n");
        }
        Item::FunctionDef {
            name,
            type_params,
            params,
            return_type,
            body,
            ..
        } => {
            indent(out, depth);
            out.push_str(&format!("fn {}", name));
            format_type_params(out, type_params);
            out.push('(');
            format_params(out, params);
            out.push(')');
            format_return_type(out, return_type);
            out.push(' ');
            format_expr_with_comments(out, body, depth, f);
            out.push('\n');
        }
    }
}

fn format_params(out: &mut String, params: &[Param]) {
    for (i, p) in params.iter().enumerate() {
        if i > 0 {
            out.push_str(", ");
        }
        out.push_str(&format!("{}: {}", p.name, format_type_annotation(&p.ty)));
    }
}

fn format_type_params(out: &mut String, tps: &[TypeParam]) {
    if tps.is_empty() {
        return;
    }
    out.push('<');
    for (i, tp) in tps.iter().enumerate() {
        if i > 0 {
            out.push_str(", ");
        }
        out.push_str(&tp.name);
        if !tp.bounds.is_empty() {
            out.push_str(": ");
            out.push_str(&tp.bounds.join(" + "));
        }
    }
    out.push('>');
}

fn format_return_type(out: &mut String, ty: &TypeAnnotation) {
    let name = format_type_annotation(ty);
    if name != "()" && !name.is_empty() {
        out.push_str(&format!(" -> {}", name));
    }
}

fn format_type_annotation(ty: &TypeAnnotation) -> String {
    match ty {
        TypeAnnotation::Named { name, .. } => name.clone(),
        TypeAnnotation::Array { elem, size, .. } => {
            format!("[{}]{}", size, format_type_annotation(elem))
        }
        TypeAnnotation::Pointer { inner, .. } => format!("*{}", format_type_annotation(inner)),
        TypeAnnotation::Tuple { elements, .. } => {
            let types: Vec<String> = elements.iter().map(|e| format_type_annotation(e)).collect();
            format!("({})", types.join(", "))
        }
        TypeAnnotation::Slice { elem, .. } => format!("[]{}", format_type_annotation(elem)),
        TypeAnnotation::Function { params, ret, .. } => {
            let ptypes: Vec<String> = params.iter().map(|p| format_type_annotation(p)).collect();
            format!("fn({}) -> {}", ptypes.join(", "), format_type_annotation(ret))
        }
        TypeAnnotation::DynTrait { trait_name, .. } => format!("dyn {}", trait_name),
        TypeAnnotation::Optional { inner, .. } => format!("?{}", format_type_annotation(inner)),
    }
}

/// Format a block expression with comment interleaving for statements.
fn format_expr_with_comments(out: &mut String, expr: &Expr, depth: usize, f: &mut Fmt) {
    if let Expr::Block {
        stmts, tail_expr, ..
    } = expr
    {
        out.push_str("{\n");
        for stmt in stmts {
            let stmt_line = f.line_of(stmt_span(stmt));
            if f.has_source() {
                f.emit_comments_before(out, stmt_line, depth + 1);
            }
            format_stmt(out, stmt, depth + 1, f);
            // Append trailing comment if one exists on this line
            if f.has_source() && out.ends_with('\n') {
                let mut trail = String::new();
                f.emit_trailing_comment(&mut trail, stmt_line);
                if !trail.is_empty() {
                    out.pop(); // remove '\n'
                    out.push_str(&trail);
                    out.push('\n');
                }
            }
        }
        if let Some(te) = tail_expr {
            indent(out, depth + 1);
            format_expr(out, te, depth);
            out.push('\n');
        }
        indent(out, depth);
        out.push('}');
    } else {
        format_expr(out, expr, depth);
    }
}

fn format_expr(out: &mut String, expr: &Expr, depth: usize) {
    match expr {
        Expr::Literal { value, .. } => match value {
            LitValue::Int(v) => out.push_str(&v.to_string()),
            LitValue::Float(v) => {
                let s = format!("{}", v);
                out.push_str(&s);
                if !s.contains('.') {
                    out.push_str(".0");
                }
            }
            LitValue::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
            LitValue::Str(s) => out.push_str(&format!("\"{}\"", s)),
            LitValue::Nil => out.push_str("nil"),
        },
        Expr::Ident { name, .. } => out.push_str(name),
        Expr::BinOp { op, lhs, rhs, .. } => {
            format_expr(out, lhs, depth);
            out.push_str(&format!(" {} ", format_binop(op)));
            format_expr(out, rhs, depth);
        }
        Expr::UnaryOp { op, operand, .. } => {
            out.push_str(format_unaryop(op));
            format_expr(out, operand, depth);
        }
        Expr::Call { callee, args, .. } => {
            out.push_str(callee);
            out.push('(');
            format_expr_list(out, args, depth);
            out.push(')');
        }
        Expr::If {
            condition,
            then_branch,
            else_branch,
            ..
        } => {
            out.push_str("if ");
            format_expr(out, condition, depth);
            out.push(' ');
            format_expr(out, then_branch, depth);
            if let Some(eb) = else_branch {
                out.push_str(" else ");
                format_expr(out, eb, depth);
            }
        }
        Expr::Block {
            stmts, tail_expr, ..
        } => {
            out.push_str("{\n");
            for stmt in stmts {
                format_stmt_no_comments(out, stmt, depth + 1);
            }
            if let Some(te) = tail_expr {
                indent(out, depth + 1);
                format_expr(out, te, depth + 1);
                out.push('\n');
            }
            indent(out, depth);
            out.push('}');
        }
        Expr::ArrayLit { elements, .. } => {
            out.push('[');
            format_expr_list(out, elements, depth);
            out.push(']');
        }
        Expr::Index { object, index, .. } => {
            format_expr(out, object, depth);
            out.push('[');
            format_expr(out, index, depth);
            out.push(']');
        }
        Expr::FieldAccess { object, field, .. } => {
            format_expr(out, object, depth);
            out.push('.');
            out.push_str(field);
        }
        Expr::StructInit { name, fields, .. } => {
            out.push_str(name);
            out.push_str(" { ");
            for (i, (fname, fexpr)) in fields.iter().enumerate() {
                if i > 0 {
                    out.push_str(", ");
                }
                out.push_str(fname);
                out.push_str(": ");
                format_expr(out, fexpr, depth);
            }
            out.push_str(" }");
        }
        Expr::New { name, fields, .. } => {
            out.push_str("new ");
            out.push_str(name);
            out.push_str(" { ");
            for (i, (fname, fexpr)) in fields.iter().enumerate() {
                if i > 0 {
                    out.push_str(", ");
                }
                out.push_str(fname);
                out.push_str(": ");
                format_expr(out, fexpr, depth);
            }
            out.push_str(" }");
        }
        Expr::AddrOf { operand, .. } => {
            out.push('&');
            format_expr(out, operand, depth);
        }
        Expr::Deref { operand, .. } => {
            out.push('*');
            format_expr(out, operand, depth);
        }
        Expr::MethodCall {
            object,
            method,
            args,
            ..
        } => {
            format_expr(out, object, depth);
            out.push('.');
            out.push_str(method);
            out.push('(');
            format_expr_list(out, args, depth);
            out.push(')');
        }
        Expr::Cast {
            expr, target_type, ..
        } => {
            format_expr(out, expr, depth);
            out.push_str(&format!(" as {}", format_type_annotation(target_type)));
        }
        Expr::Match { subject, arms, .. } => {
            out.push_str("match ");
            format_expr(out, subject, depth);
            out.push_str(" {\n");
            for arm in arms {
                indent(out, depth + 1);
                format_pattern(out, &arm.pattern);
                out.push_str(" => ");
                format_expr(out, &arm.body, depth + 1);
                out.push_str(",\n");
            }
            indent(out, depth);
            out.push('}');
        }
        Expr::TupleLit { elements, .. } => {
            out.push('(');
            format_expr_list(out, elements, depth);
            out.push(')');
        }
        Expr::TupleIndex { object, index, .. } => {
            format_expr(out, object, depth);
            out.push_str(&format!(".{}", index));
        }
        Expr::EnumVariant {
            enum_name,
            variant,
            args,
            ..
        } => {
            out.push_str(&format!("{}::{}", enum_name, variant));
            if !args.is_empty() {
                out.push('(');
                format_expr_list(out, args, depth);
                out.push(')');
            }
        }
        Expr::Try { operand, .. } => {
            format_expr(out, operand, depth);
            out.push('?');
        }
        Expr::Lambda {
            params,
            return_type,
            body,
            ..
        } => {
            out.push('|');
            format_params(out, params);
            out.push('|');
            let ret = format_type_annotation(return_type);
            if ret != "()" && !ret.is_empty() {
                out.push_str(&format!(" -> {}", ret));
            }
            out.push(' ');
            format_expr(out, body, depth);
        }
        Expr::RangeIndex {
            object, start, end, ..
        } => {
            format_expr(out, object, depth);
            out.push('[');
            format_expr(out, start, depth);
            out.push_str("..");
            format_expr(out, end, depth);
            out.push(']');
        }
        Expr::Await { future, .. } => {
            out.push_str("await ");
            format_expr(out, future, depth);
        }
        Expr::Go { call, .. } => {
            out.push_str("go ");
            format_expr(out, call, depth);
        }
        Expr::NullCoalesce { value, default, .. } => {
            format_expr(out, value, depth);
            out.push_str(" ?? ");
            format_expr(out, default, depth);
        }
    }
}

fn format_stmt(out: &mut String, stmt: &Stmt, depth: usize, f: &mut Fmt) {
    format_stmt_inner(out, stmt, depth, Some(f));
}

fn format_stmt_no_comments(out: &mut String, stmt: &Stmt, depth: usize) {
    format_stmt_inner(out, stmt, depth, None);
}

fn format_stmt_inner(out: &mut String, stmt: &Stmt, depth: usize, f: Option<&mut Fmt>) {
    match stmt {
        Stmt::VarDecl {
            name,
            mutability,
            ty,
            init,
            ..
        } => {
            indent(out, depth);
            out.push_str(name);
            match (mutability, ty) {
                (Mutability::Mutable, Some(ty)) => {
                    out.push_str(&format!(" :~ {} = ", format_type_annotation(ty)));
                }
                (Mutability::Mutable, None) => {
                    out.push_str(" :~= ");
                }
                (Mutability::Immutable, Some(ty)) => {
                    out.push_str(&format!(" : {} = ", format_type_annotation(ty)));
                }
                (Mutability::Immutable, None) => {
                    out.push_str(" := ");
                }
            }
            format_expr(out, init, depth);
            out.push_str(";\n");
        }
        Stmt::ConstDecl {
            name, ty, value, ..
        } => {
            indent(out, depth);
            if let Some(ty) = ty {
                out.push_str(&format!("{} : {} = ", name, format_type_annotation(ty)));
            } else {
                out.push_str(&format!("{} := ", name));
            }
            format_expr(out, value, depth);
            out.push_str(";\n");
        }
        Stmt::Assign { target, value, .. } => {
            indent(out, depth);
            // Detect compound assignment pattern: x = x + expr → x += expr
            if let Some((op_str, rhs)) = detect_compound_assign(target, value) {
                format_assign_target(out, target, depth);
                out.push_str(&format!(" {}= ", op_str));
                format_expr(out, rhs, depth);
            } else {
                format_assign_target(out, target, depth);
                out.push_str(" = ");
                format_expr(out, value, depth);
            }
            out.push_str(";\n");
        }
        Stmt::ExprStmt { expr, .. } => {
            indent(out, depth);
            format_expr(out, expr, depth);
            match expr {
                Expr::If { .. } | Expr::Match { .. } | Expr::Block { .. } => out.push('\n'),
                _ => out.push_str(";\n"),
            }
        }
        Stmt::Return { value, .. } => {
            indent(out, depth);
            if let Some(v) = value {
                out.push_str("return ");
                format_expr(out, v, depth);
                out.push_str(";\n");
            } else {
                out.push_str("return;\n");
            }
        }
        Stmt::While {
            condition, body, ..
        } => {
            indent(out, depth);
            out.push_str("while ");
            format_expr(out, condition, depth);
            out.push(' ');
            if let Some(f) = f {
                format_expr_with_comments(out, body, depth, f);
            } else {
                format_expr(out, body, depth);
            }
            out.push('\n');
        }
        Stmt::ForRange {
            var,
            start,
            end,
            inclusive,
            body,
            ..
        } => {
            indent(out, depth);
            out.push_str(&format!("for {} in ", var));
            format_expr(out, start, depth);
            out.push_str(if *inclusive { "..=" } else { ".." });
            format_expr(out, end, depth);
            out.push(' ');
            format_expr(out, body, depth);
            out.push('\n');
        }
        Stmt::ForIn {
            var,
            collection,
            body,
            ..
        } => {
            indent(out, depth);
            out.push_str(&format!("for {} in ", var));
            format_expr(out, collection, depth);
            out.push(' ');
            format_expr(out, body, depth);
            out.push('\n');
        }
        Stmt::Break { .. } => {
            indent(out, depth);
            out.push_str("break;\n");
        }
        Stmt::Continue { .. } => {
            indent(out, depth);
            out.push_str("continue;\n");
        }
        Stmt::TupleDestructure {
            names,
            mutability,
            init,
            ..
        } => {
            indent(out, depth);
            out.push('(');
            out.push_str(&names.join(", "));
            out.push(')');
            if *mutability == Mutability::Mutable {
                out.push_str(" :~= ");
            } else {
                out.push_str(" := ");
            }
            format_expr(out, init, depth);
            out.push_str(";\n");
        }
        Stmt::Defer { body, .. } => {
            indent(out, depth);
            out.push_str("defer ");
            format_expr(out, body, depth);
            out.push_str(";\n");
        }
        Stmt::WhileLet {
            pattern,
            expr,
            body,
            ..
        } => {
            indent(out, depth);
            out.push_str("while let ");
            format_pattern(out, pattern);
            out.push_str(" = ");
            format_expr(out, expr, depth);
            out.push(' ');
            format_expr(out, body, depth);
            out.push('\n');
        }
        Stmt::IfLet {
            pattern,
            expr,
            then_body,
            else_body,
            ..
        } => {
            indent(out, depth);
            out.push_str("if let ");
            format_pattern(out, pattern);
            out.push_str(" = ");
            format_expr(out, expr, depth);
            out.push(' ');
            format_expr(out, then_body, depth);
            if let Some(eb) = else_body {
                out.push_str(" else ");
                format_expr(out, eb, depth);
            }
            out.push('\n');
        }
        Stmt::Loop { body, .. } => {
            indent(out, depth);
            out.push_str("loop ");
            format_expr(out, body, depth);
            out.push('\n');
        }
        Stmt::ForMap { key_var, val_var, map_expr, body, .. } => {
            indent(out, depth);
            out.push_str(&format!("for {}, {} in ", key_var, val_var));
            format_expr(out, map_expr, depth);
            out.push(' ');
            format_expr(out, body, depth);
            out.push('\n');
        }
        Stmt::Select { arms, .. } => {
            indent(out, depth);
            out.push_str("select {\n");
            for arm in arms {
                indent(out, depth + 1);
                out.push_str(&arm.var);
                out.push_str(" := ");
                format_expr(out, &arm.channel, depth + 1);
                out.push_str(" => ");
                format_expr(out, &arm.body, depth + 1);
                out.push_str(",\n");
            }
            indent(out, depth);
            out.push_str("}\n");
        }
    }
}

fn format_pattern(out: &mut String, pattern: &Pattern) {
    match pattern {
        Pattern::EnumVariant {
            enum_name,
            variant,
            bindings,
            ..
        } => {
            out.push_str(&format!("{}::{}", enum_name, variant));
            if !bindings.is_empty() {
                out.push('(');
                out.push_str(&bindings.join(", "));
                out.push(')');
            }
        }
        Pattern::IntLit(v, _) => out.push_str(&v.to_string()),
        Pattern::Wildcard(_) => out.push('_'),
    }
}

fn format_assign_target(out: &mut String, target: &AssignTarget, depth: usize) {
    match target {
        AssignTarget::Var(name) => out.push_str(name),
        AssignTarget::Index(obj, idx) => {
            format_expr(out, obj, depth);
            out.push('[');
            format_expr(out, idx, depth);
            out.push(']');
        }
        AssignTarget::Field(obj, field) => {
            format_expr(out, obj, depth);
            out.push('.');
            out.push_str(field);
        }
        AssignTarget::Deref(obj) => {
            out.push('*');
            format_expr(out, obj, depth);
        }
    }
}

/// Detect compound assignment: `x = x op expr` → returns (op_str, expr)
fn detect_compound_assign<'a>(target: &AssignTarget, value: &'a Expr) -> Option<(&'static str, &'a Expr)> {
    let target_name = match target {
        AssignTarget::Var(name) => name.as_str(),
        _ => return None,
    };
    if let Expr::BinOp { op, lhs, rhs, .. } = value {
        if let Expr::Ident { name, .. } = lhs.as_ref() {
            if name == target_name {
                let op_str = match op {
                    BinOp::Add => "+",
                    BinOp::Sub => "-",
                    BinOp::Mul => "*",
                    BinOp::Div => "/",
                    BinOp::Mod => "%",
                    BinOp::BitAnd => "&",
                    BinOp::BitOr => "|",
                    BinOp::BitXor => "^",
                    BinOp::Shl => "<<",
                    BinOp::Shr => ">>",
                    _ => return None,
                };
                return Some((op_str, rhs.as_ref()));
            }
        }
    }
    None
}

fn format_expr_list(out: &mut String, exprs: &[Expr], depth: usize) {
    for (i, e) in exprs.iter().enumerate() {
        if i > 0 {
            out.push_str(", ");
        }
        format_expr(out, e, depth);
    }
}

fn format_binop(op: &BinOp) -> &'static str {
    match op {
        BinOp::Add => "+",
        BinOp::Sub => "-",
        BinOp::Mul => "*",
        BinOp::Div => "/",
        BinOp::Mod => "%",
        BinOp::Eq => "==",
        BinOp::Ne => "!=",
        BinOp::Lt => "<",
        BinOp::Gt => ">",
        BinOp::Le => "<=",
        BinOp::Ge => ">=",
        BinOp::And => "&&",
        BinOp::Or => "||",
        BinOp::BitAnd => "&",
        BinOp::BitOr => "|",
        BinOp::BitXor => "^",
        BinOp::Shl => "<<",
        BinOp::Shr => ">>",
    }
}

fn format_unaryop(op: &UnaryOp) -> &'static str {
    match op {
        UnaryOp::Neg => "-",
        UnaryOp::Not => "!",
        UnaryOp::BitNot => "~",
    }
}

fn indent(out: &mut String, depth: usize) {
    for _ in 0..depth {
        out.push_str(INDENT);
    }
}
