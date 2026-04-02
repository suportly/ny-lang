pub mod ast;
pub mod precedence;

use crate::common::{CompileError, Span};
use crate::lexer::token::{Token, TokenKind};
use ast::*;
use precedence::{infix_bp, prefix_bp};

/// Parse a single expression from a source fragment (used for f-string interpolation).
fn parse_expression_from_source(source: &str, span: Span) -> Result<Expr, CompileError> {
    let tokens = crate::lexer::tokenize(source).map_err(|errs| {
        let mut e = errs.into_iter().next().unwrap();
        e.message = format!("in f-string expression: {}", e.message);
        e.span = span;
        e
    })?;
    let mut parser = Parser::new(tokens);
    parser.parse_expr(0).map_err(|mut e| {
        e.message = format!("in f-string expression: {}", e.message);
        e.span = span;
        e
    })
}

pub struct Parser {
    tokens: Vec<Token>,
    pos: usize,
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Self { tokens, pos: 0 }
    }

    fn peek(&self) -> &TokenKind {
        &self.tokens[self.pos].kind
    }

    fn peek_span(&self) -> Span {
        self.tokens[self.pos].span
    }

    fn advance(&mut self) -> &Token {
        let t = &self.tokens[self.pos];
        if self.pos < self.tokens.len() - 1 {
            self.pos += 1;
        }
        t
    }

    fn expect(&mut self, expected: &TokenKind) -> Result<Span, CompileError> {
        if self.peek() == expected {
            Ok(self.advance().span)
        } else {
            Err(CompileError::syntax(
                format!("expected {:?}, found {:?}", expected, self.peek()),
                self.peek_span(),
            ))
        }
    }

    fn expect_ident(&mut self) -> Result<(String, Span), CompileError> {
        match self.peek().clone() {
            TokenKind::Ident(name) => {
                let name = name.clone();
                let span = self.advance().span;
                Ok((name, span))
            }
            _ => Err(CompileError::syntax(
                format!("expected identifier, found {:?}", self.peek()),
                self.peek_span(),
            )),
        }
    }

    /// Parse optional trait bounds after a type parameter name: `: Trait1 + Trait2`
    fn parse_type_param_bounds(&mut self) -> Result<Vec<String>, CompileError> {
        let mut bounds = Vec::new();
        if *self.peek() == TokenKind::Colon {
            self.advance(); // consume :
            loop {
                let (trait_name, _) = self.expect_ident()?;
                bounds.push(trait_name);
                if *self.peek() != TokenKind::Plus {
                    break;
                }
                self.advance(); // consume +
            }
        }
        Ok(bounds)
    }

    fn parse_type_annotation(&mut self) -> Result<TypeAnnotation, CompileError> {
        let start = self.peek_span();

        // Pointer type: *T
        if *self.peek() == TokenKind::Star {
            self.advance();
            let inner = self.parse_type_annotation()?;
            let span = start.merge(inner.span());
            return Ok(TypeAnnotation::Pointer {
                inner: Box::new(inner),
                span,
            });
        }

        // Array type: [N]T or Slice type: []T
        if *self.peek() == TokenKind::LBracket {
            self.advance();
            // Check for slice: []T (no size)
            if *self.peek() == TokenKind::RBracket {
                self.advance();
                let elem = self.parse_type_annotation()?;
                let span = start.merge(elem.span());
                return Ok(TypeAnnotation::Slice {
                    elem: Box::new(elem),
                    span,
                });
            }
            let size = match self.peek().clone() {
                TokenKind::IntLit(n) => {
                    self.advance();
                    n as usize
                }
                _ => {
                    return Err(CompileError::syntax(
                        "expected array size (integer literal) or ']' for slice".to_string(),
                        self.peek_span(),
                    ));
                }
            };
            self.expect(&TokenKind::RBracket)?;
            let elem = self.parse_type_annotation()?;
            let span = start.merge(elem.span());
            return Ok(TypeAnnotation::Array {
                elem: Box::new(elem),
                size,
                span,
            });
        }

        // Tuple type: (T1, T2, ...)
        if *self.peek() == TokenKind::LParen {
            self.advance();
            let mut elements = Vec::new();
            while *self.peek() != TokenKind::RParen {
                if !elements.is_empty() {
                    self.expect(&TokenKind::Comma)?;
                }
                elements.push(Box::new(self.parse_type_annotation()?));
            }
            let end = self.peek_span();
            self.expect(&TokenKind::RParen)?;
            return Ok(TypeAnnotation::Tuple {
                elements,
                span: start.merge(end),
            });
        }

        // Function pointer type: fn(T1, T2) -> R
        if *self.peek() == TokenKind::Fn {
            let fn_start = self.advance().span;
            self.expect(&TokenKind::LParen)?;
            let mut param_types = Vec::new();
            while *self.peek() != TokenKind::RParen {
                if !param_types.is_empty() {
                    self.expect(&TokenKind::Comma)?;
                }
                param_types.push(Box::new(self.parse_type_annotation()?));
            }
            self.expect(&TokenKind::RParen)?;
            self.expect(&TokenKind::Arrow)?;
            let ret_type = Box::new(self.parse_type_annotation()?);
            let span = fn_start.merge(ret_type.span());
            return Ok(TypeAnnotation::Function {
                params: param_types,
                ret: ret_type,
                span,
            });
        }

        // Named type (possibly generic: Vec<i32>)
        let (name, span) = self.expect_ident()?;

        // Check for generic type args: Name<T1, T2>
        // In type annotation context, < is always a generic delimiter (not comparison)
        // so we can safely parse it for any capitalized type name
        let starts_upper = name.chars().next().map_or(false, |c| c.is_uppercase());
        if starts_upper && *self.peek() == TokenKind::Lt {
            self.advance(); // consume <
            let mut type_args = Vec::new();
            while *self.peek() != TokenKind::Gt {
                if !type_args.is_empty() {
                    self.expect(&TokenKind::Comma)?;
                }
                type_args.push(self.parse_type_annotation()?);
            }
            let end = self.peek_span();
            self.expect(&TokenKind::Gt)?;

            // For Vec<T>, encode as a special name
            if name == "Vec" && type_args.len() == 1 {
                return Ok(TypeAnnotation::Named {
                    name: format!("Vec<{}>", type_args[0].name_str()),
                    span: span.merge(end),
                });
            }

            // Generic named type — encode the full name for now
            let args_str: Vec<String> =
                type_args.iter().map(|a| a.name_str().to_string()).collect();
            return Ok(TypeAnnotation::Named {
                name: format!("{}<{}>", name, args_str.join(",")),
                span: span.merge(end),
            });
        }

        Ok(TypeAnnotation::Named { name, span })
    }

    fn parse_program(&mut self) -> Result<Program, Vec<CompileError>> {
        let mut items = Vec::new();
        let mut errors = Vec::new();

        while *self.peek() != TokenKind::Eof {
            match self.parse_item() {
                Ok(item) => items.push(item),
                Err(e) => {
                    errors.push(e);
                    while !matches!(
                        self.peek(),
                        TokenKind::Fn
                            | TokenKind::Struct
                            | TokenKind::Enum
                            | TokenKind::Impl
                            | TokenKind::Trait
                            | TokenKind::Extern
                            | TokenKind::Use
                            | TokenKind::Eof
                    ) {
                        self.advance();
                    }
                }
            }
        }

        if errors.is_empty() {
            Ok(Program { items })
        } else {
            Err(errors)
        }
    }

    fn parse_item(&mut self) -> Result<Item, CompileError> {
        // Skip optional `pub` keyword
        if *self.peek() == TokenKind::Pub {
            self.advance();
        }
        match self.peek() {
            TokenKind::Fn => self.parse_function(),
            TokenKind::Struct => self.parse_struct_def(),
            TokenKind::Enum => self.parse_enum_def(),
            TokenKind::Impl => self.parse_impl_block(),
            TokenKind::Trait => self.parse_trait_def(),
            TokenKind::Use => self.parse_use_decl(),
            TokenKind::Extern => self.parse_extern_block(),
            _ => Err(CompileError::syntax(
                format!(
                    "expected 'fn', 'struct', 'enum', or 'impl', found {:?}",
                    self.peek()
                ),
                self.peek_span(),
            )),
        }
    }

    fn parse_struct_def(&mut self) -> Result<Item, CompileError> {
        let start = self.peek_span();
        self.expect(&TokenKind::Struct)?;
        let (name, _) = self.expect_ident()?;

        // Optional type parameters: struct Pair<A, B> { ... }
        let mut type_params = Vec::new();
        if *self.peek() == TokenKind::Lt {
            self.advance();
            while *self.peek() != TokenKind::Gt {
                if !type_params.is_empty() {
                    self.expect(&TokenKind::Comma)?;
                }
                let (tp_name, tp_span) = self.expect_ident()?;
                let bounds = self.parse_type_param_bounds()?;
                type_params.push(TypeParam {
                    name: tp_name,
                    bounds,
                    span: tp_span,
                });
            }
            self.expect(&TokenKind::Gt)?;
        }

        self.expect(&TokenKind::LBrace)?;

        let mut fields = Vec::new();
        while *self.peek() != TokenKind::RBrace {
            let (field_name, _) = self.expect_ident()?;
            self.expect(&TokenKind::Colon)?;
            let ty = self.parse_type_annotation()?;
            fields.push((field_name, ty));
            if *self.peek() == TokenKind::Comma {
                self.advance();
            }
        }

        let end = self.peek_span();
        self.expect(&TokenKind::RBrace)?;
        Ok(Item::StructDef {
            name,
            type_params,
            fields,
            span: start.merge(end),
        })
    }

    fn parse_enum_def(&mut self) -> Result<Item, CompileError> {
        let start = self.peek_span();
        self.expect(&TokenKind::Enum)?;
        let (name, _) = self.expect_ident()?;

        // Optional type parameters: enum Option<T> { ... }
        let mut type_params = Vec::new();
        if *self.peek() == TokenKind::Lt {
            self.advance();
            while *self.peek() != TokenKind::Gt {
                if !type_params.is_empty() {
                    self.expect(&TokenKind::Comma)?;
                }
                let (tp_name, tp_span) = self.expect_ident()?;
                let bounds = self.parse_type_param_bounds()?;
                type_params.push(TypeParam {
                    name: tp_name,
                    bounds,
                    span: tp_span,
                });
            }
            self.expect(&TokenKind::Gt)?;
        }

        self.expect(&TokenKind::LBrace)?;

        let mut variants = Vec::new();
        while *self.peek() != TokenKind::RBrace {
            let var_start = self.peek_span();
            let (variant_name, _) = self.expect_ident()?;
            // Check for payload: Variant(Type1, Type2, ...)
            let mut payload = Vec::new();
            if *self.peek() == TokenKind::LParen {
                self.advance();
                while *self.peek() != TokenKind::RParen {
                    if !payload.is_empty() {
                        self.expect(&TokenKind::Comma)?;
                    }
                    payload.push(self.parse_type_annotation()?);
                }
                self.expect(&TokenKind::RParen)?;
            }
            let var_span = var_start.merge(self.tokens[self.pos.saturating_sub(1)].span);
            variants.push(EnumVariantDef {
                name: variant_name,
                payload,
                span: var_span,
            });
            if *self.peek() == TokenKind::Comma {
                self.advance();
            }
        }

        let end = self.peek_span();
        self.expect(&TokenKind::RBrace)?;
        Ok(Item::EnumDef {
            name,
            type_params,
            variants,
            span: start.merge(end),
        })
    }

    fn parse_extern_block(&mut self) -> Result<Item, CompileError> {
        let start = self.peek_span();
        self.expect(&TokenKind::Extern)?;
        self.expect(&TokenKind::LBrace)?;

        let mut functions = Vec::new();
        while *self.peek() != TokenKind::RBrace && *self.peek() != TokenKind::Eof {
            let fn_start = self.peek_span();
            self.expect(&TokenKind::Fn)?;
            let (name, _) = self.expect_ident()?;
            self.expect(&TokenKind::LParen)?;

            let mut params = Vec::new();
            let mut variadic = false;
            while *self.peek() != TokenKind::RParen {
                if !params.is_empty() {
                    self.expect(&TokenKind::Comma)?;
                }
                // Check for variadic: ...
                if *self.peek() == TokenKind::DotDot {
                    self.advance();
                    if *self.peek() == TokenKind::Dot {
                        self.advance();
                    }
                    variadic = true;
                    break;
                }
                let param_start = self.peek_span();
                let (param_name, _) = self.expect_ident()?;
                self.expect(&TokenKind::Colon)?;
                let ty = self.parse_type_annotation()?;
                let param_span = param_start.merge(ty.span());
                params.push(Param {
                    name: param_name,
                    ty,
                    span: param_span,
                });
            }
            self.expect(&TokenKind::RParen)?;

            let return_type = if *self.peek() == TokenKind::Arrow {
                self.advance();
                self.parse_type_annotation()?
            } else {
                TypeAnnotation::Named {
                    name: "()".to_string(),
                    span: self.peek_span(),
                }
            };

            let fn_end = self.peek_span();
            self.expect(&TokenKind::Semi)?;
            functions.push(ExternFnDecl {
                name,
                params,
                return_type,
                variadic,
                span: fn_start.merge(fn_end),
            });
        }

        let end = self.peek_span();
        self.expect(&TokenKind::RBrace)?;
        Ok(Item::ExternBlock {
            functions,
            span: start.merge(end),
        })
    }

    fn parse_use_decl(&mut self) -> Result<Item, CompileError> {
        let start = self.peek_span();
        self.expect(&TokenKind::Use)?;
        // use "path/to/module.ny";
        let path = match self.peek().clone() {
            TokenKind::StringLit(s) => {
                let s = s.clone();
                self.advance();
                s
            }
            TokenKind::Ident(name) => {
                // use module_name; → resolves to module_name.ny
                let name = name.clone();
                self.advance();
                format!("{}.ny", name)
            }
            _ => {
                return Err(CompileError::syntax(
                    format!("expected module path after 'use', found {:?}", self.peek()),
                    self.peek_span(),
                ));
            }
        };
        let end = self.peek_span();
        self.expect(&TokenKind::Semi)?;
        Ok(Item::Use {
            path,
            span: start.merge(end),
        })
    }

    fn parse_trait_def(&mut self) -> Result<Item, CompileError> {
        let start = self.peek_span();
        self.expect(&TokenKind::Trait)?;
        let (name, _) = self.expect_ident()?;
        self.expect(&TokenKind::LBrace)?;

        let mut methods = Vec::new();
        while *self.peek() != TokenKind::RBrace && *self.peek() != TokenKind::Eof {
            let sig_start = self.peek_span();
            self.expect(&TokenKind::Fn)?;
            let (method_name, _) = self.expect_ident()?;
            self.expect(&TokenKind::LParen)?;

            let mut params = Vec::new();
            while *self.peek() != TokenKind::RParen {
                if !params.is_empty() {
                    self.expect(&TokenKind::Comma)?;
                }
                let param_start = self.peek_span();
                let (param_name, _) = self.expect_ident()?;
                self.expect(&TokenKind::Colon)?;
                let ty = self.parse_type_annotation()?;
                let param_span = param_start.merge(ty.span());
                params.push(Param {
                    name: param_name,
                    ty,
                    span: param_span,
                });
            }
            self.expect(&TokenKind::RParen)?;
            self.expect(&TokenKind::Arrow)?;
            let return_type = self.parse_type_annotation()?;
            let end = self.peek_span();
            self.expect(&TokenKind::Semi)?;
            methods.push(TraitMethodSig {
                name: method_name,
                params,
                return_type,
                span: sig_start.merge(end),
            });
        }

        let end = self.peek_span();
        self.expect(&TokenKind::RBrace)?;
        Ok(Item::TraitDef {
            name,
            methods,
            span: start.merge(end),
        })
    }

    fn parse_impl_block(&mut self) -> Result<Item, CompileError> {
        let start = self.peek_span();
        self.expect(&TokenKind::Impl)?;
        let (first_name, _) = self.expect_ident()?;

        // Check for `impl Trait for Type { ... }`
        let (trait_name, type_name) = if *self.peek() == TokenKind::For {
            self.advance(); // consume "for"
            let (tn, _) = self.expect_ident()?;
            (Some(first_name), tn)
        } else {
            (None, first_name)
        };

        self.expect(&TokenKind::LBrace)?;

        let mut methods = Vec::new();
        while *self.peek() != TokenKind::RBrace && *self.peek() != TokenKind::Eof {
            // Skip optional pub inside impl
            if *self.peek() == TokenKind::Pub {
                self.advance();
            }
            methods.push(self.parse_function()?);
        }

        let end = self.peek_span();
        self.expect(&TokenKind::RBrace)?;
        Ok(Item::ImplBlock {
            type_name,
            trait_name,
            methods,
            span: start.merge(end),
        })
    }

    fn parse_function(&mut self) -> Result<Item, CompileError> {
        let start_span = self.peek_span();
        self.expect(&TokenKind::Fn)?;
        let (name, _) = self.expect_ident()?;

        // Parse optional type parameters: <T, U, ...> or <T: Trait, U: A + B>
        let mut type_params = Vec::new();
        if *self.peek() == TokenKind::Lt {
            self.advance(); // consume <
            while *self.peek() != TokenKind::Gt {
                if !type_params.is_empty() {
                    self.expect(&TokenKind::Comma)?;
                }
                let (tp_name, tp_span) = self.expect_ident()?;
                let bounds = self.parse_type_param_bounds()?;
                type_params.push(TypeParam {
                    name: tp_name,
                    bounds,
                    span: tp_span,
                });
            }
            self.expect(&TokenKind::Gt)?;
        }

        self.expect(&TokenKind::LParen)?;

        let mut params = Vec::new();
        while *self.peek() != TokenKind::RParen {
            if !params.is_empty() {
                self.expect(&TokenKind::Comma)?;
            }
            let param_start = self.peek_span();
            let (param_name, _) = self.expect_ident()?;
            self.expect(&TokenKind::Colon)?;
            let ty = self.parse_type_annotation()?;
            let param_span = param_start.merge(ty.span());
            params.push(Param {
                name: param_name,
                ty,
                span: param_span,
            });
        }
        self.expect(&TokenKind::RParen)?;

        // Return type: -> T or implicit () if no arrow
        let return_type = if *self.peek() == TokenKind::Arrow {
            self.advance();
            self.parse_type_annotation()?
        } else {
            TypeAnnotation::Named {
                name: "()".to_string(),
                span: self.peek_span(),
            }
        };

        let body = if *self.peek() == TokenKind::Assign {
            self.advance();
            let expr = self.parse_expr(0)?;
            self.expect(&TokenKind::Semi)?;
            let span = expr.span();
            Expr::Block {
                stmts: vec![Stmt::Return {
                    value: Some(expr),
                    span,
                }],
                tail_expr: None,
                span,
            }
        } else {
            self.parse_block_expr()?
        };

        let end_span = body.span();
        Ok(Item::FunctionDef {
            name,
            type_params,
            params,
            return_type,
            body,
            span: start_span.merge(end_span),
        })
    }

    fn parse_block_expr(&mut self) -> Result<Expr, CompileError> {
        let start = self.peek_span();
        self.expect(&TokenKind::LBrace)?;

        let mut stmts = Vec::new();
        let mut tail_expr = None;

        while *self.peek() != TokenKind::RBrace && *self.peek() != TokenKind::Eof {
            match self.peek() {
                TokenKind::Return => stmts.push(self.parse_return_stmt()?),
                TokenKind::While => stmts.push(self.parse_while_stmt()?),
                TokenKind::For => stmts.push(self.parse_for_stmt()?),
                TokenKind::Break => {
                    let span = self.advance().span;
                    self.expect(&TokenKind::Semi)?;
                    stmts.push(Stmt::Break { span });
                }
                TokenKind::Continue => {
                    let span = self.advance().span;
                    self.expect(&TokenKind::Semi)?;
                    stmts.push(Stmt::Continue { span });
                }
                TokenKind::Defer => {
                    stmts.push(self.parse_defer_stmt()?);
                }
                TokenKind::Loop => {
                    stmts.push(self.parse_loop_stmt()?);
                }
                TokenKind::LParen if self.is_tuple_destructure() => {
                    stmts.push(self.parse_tuple_destructure()?);
                }
                TokenKind::Ident(_) => {
                    if self.is_var_decl_or_assign() {
                        stmts.push(self.parse_var_decl_or_assign()?);
                    } else {
                        self.parse_expr_or_assign_stmt(&mut stmts, &mut tail_expr)?;
                    }
                }
                TokenKind::If => {
                    let expr = self.parse_if_expr()?;
                    self.handle_expr_in_block(expr, &mut stmts, &mut tail_expr)?;
                }
                _ => {
                    self.parse_expr_or_assign_stmt(&mut stmts, &mut tail_expr)?;
                }
            }
        }

        let end = self.peek_span();
        self.expect(&TokenKind::RBrace)?;
        Ok(Expr::Block {
            stmts,
            tail_expr,
            span: start.merge(end),
        })
    }

    fn parse_expr_or_assign_stmt(
        &mut self,
        stmts: &mut Vec<Stmt>,
        tail_expr: &mut Option<Box<Expr>>,
    ) -> Result<(), CompileError> {
        let expr = self.parse_expr(0)?;

        // Check for assignment: lhs = rhs; or compound assignment: lhs += rhs;
        let is_assign = *self.peek() == TokenKind::Assign;
        let compound_op = compound_to_binop(self.peek());

        if is_assign || compound_op.is_some() {
            self.advance();
            let rhs = self.parse_expr(0)?;

            // For compound assignment, desugar: target op= rhs → target = target op rhs
            let value = if let Some(op) = compound_op {
                let target_expr = expr.clone();
                let span = target_expr.span().merge(rhs.span());
                Expr::BinOp {
                    op,
                    lhs: Box::new(target_expr),
                    rhs: Box::new(rhs),
                    span,
                }
            } else {
                rhs
            };

            let span = expr.span().merge(value.span());
            let end = self.peek_span();
            self.expect(&TokenKind::Semi)?;
            let target = match expr {
                Expr::FieldAccess { object, field, .. } => AssignTarget::Field(object, field),
                Expr::Index { object, index, .. } => AssignTarget::Index(object, index),
                Expr::Deref { operand, .. } => AssignTarget::Deref(operand),
                Expr::Ident { name, .. } => AssignTarget::Var(name),
                _ => {
                    return Err(CompileError::syntax(
                        "invalid assignment target".to_string(),
                        expr.span(),
                    ));
                }
            };
            stmts.push(Stmt::Assign {
                target,
                value,
                span: span.merge(end),
            });
            Ok(())
        } else {
            self.handle_expr_in_block(expr, stmts, tail_expr)
        }
    }

    fn handle_expr_in_block(
        &mut self,
        expr: Expr,
        stmts: &mut Vec<Stmt>,
        tail_expr: &mut Option<Box<Expr>>,
    ) -> Result<(), CompileError> {
        if *self.peek() == TokenKind::Semi {
            let span = expr.span();
            self.advance();
            stmts.push(Stmt::ExprStmt { expr, span });
        } else if *self.peek() == TokenKind::RBrace {
            *tail_expr = Some(Box::new(expr));
        } else {
            let span = expr.span();
            stmts.push(Stmt::ExprStmt { expr, span });
        }
        Ok(())
    }

    fn is_var_decl_or_assign(&self) -> bool {
        if let TokenKind::Ident(_) = &self.tokens[self.pos].kind {
            if self.pos + 1 < self.tokens.len() {
                match &self.tokens[self.pos + 1].kind {
                    TokenKind::Colon
                    | TokenKind::ColonTilde
                    | TokenKind::ColonAssign
                    | TokenKind::ColonTildeAssign
                    | TokenKind::Assign => true,
                    // ColonColon could be const decl (name :: Type = value)
                    // or enum variant (EnumName::Variant). Distinguish by checking
                    // if token at pos+3 is Assign (const decl) vs anything else (enum variant).
                    TokenKind::ColonColon => {
                        if self.pos + 3 < self.tokens.len() {
                            // For simple named types: name :: Type = ...
                            matches!(self.tokens[self.pos + 3].kind, TokenKind::Assign)
                        } else {
                            false
                        }
                    }
                    _ => false,
                }
            } else {
                false
            }
        } else {
            false
        }
    }

    fn parse_var_decl_or_assign(&mut self) -> Result<Stmt, CompileError> {
        let start = self.peek_span();
        let (name, _) = self.expect_ident()?;

        match self.peek().clone() {
            TokenKind::Colon => {
                self.advance();
                let ty = self.parse_type_annotation()?;
                self.expect(&TokenKind::Assign)?;
                let init = self.parse_expr(0)?;
                let end = self.peek_span();
                self.expect(&TokenKind::Semi)?;
                Ok(Stmt::VarDecl {
                    name,
                    mutability: Mutability::Immutable,
                    ty: Some(ty),
                    init,
                    span: start.merge(end),
                })
            }
            TokenKind::ColonTilde => {
                self.advance();
                let ty = self.parse_type_annotation()?;
                self.expect(&TokenKind::Assign)?;
                let init = self.parse_expr(0)?;
                let end = self.peek_span();
                self.expect(&TokenKind::Semi)?;
                Ok(Stmt::VarDecl {
                    name,
                    mutability: Mutability::Mutable,
                    ty: Some(ty),
                    init,
                    span: start.merge(end),
                })
            }
            TokenKind::ColonColon => {
                self.advance();
                let ty = self.parse_type_annotation()?;
                self.expect(&TokenKind::Assign)?;
                let value = self.parse_expr(0)?;
                let end = self.peek_span();
                self.expect(&TokenKind::Semi)?;
                Ok(Stmt::ConstDecl {
                    name,
                    ty: Some(ty),
                    value,
                    span: start.merge(end),
                })
            }
            TokenKind::ColonAssign => {
                self.advance();
                let init = self.parse_expr(0)?;
                let end = self.peek_span();
                self.expect(&TokenKind::Semi)?;
                Ok(Stmt::VarDecl {
                    name,
                    mutability: Mutability::Immutable,
                    ty: None,
                    init,
                    span: start.merge(end),
                })
            }
            TokenKind::ColonTildeAssign => {
                self.advance();
                let init = self.parse_expr(0)?;
                let end = self.peek_span();
                self.expect(&TokenKind::Semi)?;
                Ok(Stmt::VarDecl {
                    name,
                    mutability: Mutability::Mutable,
                    ty: None,
                    init,
                    span: start.merge(end),
                })
            }
            TokenKind::Assign => {
                self.advance();
                let value = self.parse_expr(0)?;
                let end = self.peek_span();
                self.expect(&TokenKind::Semi)?;
                Ok(Stmt::Assign {
                    target: AssignTarget::Var(name),
                    value,
                    span: start.merge(end),
                })
            }
            _ => Err(CompileError::syntax(
                format!(
                    "expected ':', ':~', '::', ':=', ':~=', or '=' after identifier, found {:?}",
                    self.peek()
                ),
                self.peek_span(),
            )),
        }
    }

    fn parse_return_stmt(&mut self) -> Result<Stmt, CompileError> {
        let start = self.peek_span();
        self.expect(&TokenKind::Return)?;
        let value = if *self.peek() != TokenKind::Semi {
            Some(self.parse_expr(0)?)
        } else {
            None
        };
        let end = self.peek_span();
        self.expect(&TokenKind::Semi)?;
        Ok(Stmt::Return {
            value,
            span: start.merge(end),
        })
    }

    fn parse_while_stmt(&mut self) -> Result<Stmt, CompileError> {
        let start = self.peek_span();
        self.expect(&TokenKind::While)?;

        // Check for while let
        if *self.peek() == TokenKind::Let {
            self.expect(&TokenKind::Let)?;
            let pattern = self.parse_pattern()?;
            self.expect(&TokenKind::Assign)?;
            let expr = self.parse_expr(0)?;
            let body = self.parse_block_expr()?;
            let end = body.span();
            return Ok(Stmt::WhileLet {
                pattern,
                expr,
                body,
                span: start.merge(end),
            });
        }

        let condition = self.parse_expr(0)?;
        let body = self.parse_block_expr()?;
        let end = body.span();
        Ok(Stmt::While {
            condition,
            body,
            span: start.merge(end),
        })
    }

    fn parse_for_stmt(&mut self) -> Result<Stmt, CompileError> {
        let start = self.peek_span();
        self.expect(&TokenKind::For)?;
        let (var, _) = self.expect_ident()?;
        self.expect(&TokenKind::In)?;
        let first_expr = self.parse_expr(0)?;

        // Check if this is a range (first_expr .. end) or for-in (first_expr is collection)
        match self.peek() {
            TokenKind::DotDot => {
                self.advance();
                let range_end = self.parse_expr(0)?;
                let body = self.parse_block_expr()?;
                let end = body.span();
                Ok(Stmt::ForRange {
                    var,
                    start: first_expr,
                    end: range_end,
                    inclusive: false,
                    body,
                    span: start.merge(end),
                })
            }
            TokenKind::DotDotEq => {
                self.advance();
                let range_end = self.parse_expr(0)?;
                let body = self.parse_block_expr()?;
                let end = body.span();
                Ok(Stmt::ForRange {
                    var,
                    start: first_expr,
                    end: range_end,
                    inclusive: true,
                    body,
                    span: start.merge(end),
                })
            }
            TokenKind::LBrace => {
                // for item in collection { body }
                let body = self.parse_block_expr()?;
                let end = body.span();
                Ok(Stmt::ForIn {
                    var,
                    collection: first_expr,
                    body,
                    span: start.merge(end),
                })
            }
            _ => Err(CompileError::syntax(
                format!(
                    "expected '..', '..=', or '{{' in for statement, found {:?}",
                    self.peek()
                ),
                self.peek_span(),
            )),
        }
    }

    fn parse_loop_stmt(&mut self) -> Result<Stmt, CompileError> {
        let start = self.peek_span();
        self.expect(&TokenKind::Loop)?;
        let body = self.parse_block_expr()?;
        let span = start.merge(body.span());
        Ok(Stmt::Loop { body, span })
    }

    fn parse_defer_stmt(&mut self) -> Result<Stmt, CompileError> {
        let start = self.peek_span();
        self.expect(&TokenKind::Defer)?;
        let body = if *self.peek() == TokenKind::LBrace {
            self.parse_block_expr()?
        } else {
            let expr = self.parse_expr(0)?;
            let end = self.peek_span();
            self.expect(&TokenKind::Semi)?;
            let span = expr.span().merge(end);
            Expr::Block {
                stmts: vec![Stmt::ExprStmt {
                    expr: expr.clone(),
                    span: expr.span(),
                }],
                tail_expr: None,
                span,
            }
        };
        let span = start.merge(body.span());
        Ok(Stmt::Defer { body, span })
    }

    fn parse_if_expr(&mut self) -> Result<Expr, CompileError> {
        let start = self.peek_span();
        self.expect(&TokenKind::If)?;

        // Check for `if let Pattern = expr { ... }`
        if *self.peek() == TokenKind::Let {
            return self.parse_if_let_expr(start);
        }

        let condition = Box::new(self.parse_expr(0)?);
        let then_branch = Box::new(self.parse_block_expr()?);

        let else_branch = if *self.peek() == TokenKind::Else {
            self.advance();
            if *self.peek() == TokenKind::If {
                Some(Box::new(self.parse_if_expr()?))
            } else {
                Some(Box::new(self.parse_block_expr()?))
            }
        } else {
            None
        };

        let end = else_branch
            .as_ref()
            .map(|e| e.span())
            .unwrap_or_else(|| then_branch.span());

        Ok(Expr::If {
            condition,
            then_branch,
            else_branch,
            span: start.merge(end),
        })
    }

    fn parse_if_let_expr(&mut self, start: Span) -> Result<Expr, CompileError> {
        self.expect(&TokenKind::Let)?;
        let pattern = self.parse_pattern()?;
        self.expect(&TokenKind::Assign)?;
        let expr = self.parse_expr(0)?;
        let then_body = self.parse_block_expr()?;

        let else_body = if *self.peek() == TokenKind::Else {
            self.advance();
            Some(self.parse_block_expr()?)
        } else {
            None
        };

        let end = else_body
            .as_ref()
            .map(|e| e.span())
            .unwrap_or_else(|| then_body.span());

        // Desugar: wrap as a statement in a block
        // This becomes: match expr { pattern => then, _ => else }
        Ok(Expr::Block {
            stmts: vec![Stmt::IfLet {
                pattern,
                expr,
                then_body,
                else_body,
                span: start.merge(end),
            }],
            tail_expr: None,
            span: start.merge(end),
        })
    }

    pub(crate) fn parse_expr(&mut self, min_bp: u8) -> Result<Expr, CompileError> {
        let mut lhs = self.parse_prefix()?;

        loop {
            if *self.peek() == TokenKind::Eof {
                break;
            }

            // Postfix: . (field access / method call), [ (index)
            match self.peek() {
                TokenKind::Dot => {
                    self.advance();
                    // Tuple index: expr.0, expr.1, etc.
                    if let TokenKind::IntLit(n) = self.peek().clone() {
                        let idx_span = self.advance().span;
                        let span = lhs.span().merge(idx_span);
                        lhs = Expr::TupleIndex {
                            object: Box::new(lhs),
                            index: n as usize,
                            span,
                        };
                        continue;
                    }
                    let (field, field_span) = self.expect_ident()?;
                    if *self.peek() == TokenKind::LParen {
                        self.advance();
                        let mut args = Vec::new();
                        while *self.peek() != TokenKind::RParen {
                            if !args.is_empty() {
                                self.expect(&TokenKind::Comma)?;
                            }
                            args.push(self.parse_expr(0)?);
                        }
                        let end = self.peek_span();
                        self.expect(&TokenKind::RParen)?;
                        let span = lhs.span().merge(end);
                        lhs = Expr::MethodCall {
                            object: Box::new(lhs),
                            method: field,
                            args,
                            span,
                        };
                    } else {
                        let span = lhs.span().merge(field_span);
                        lhs = Expr::FieldAccess {
                            object: Box::new(lhs),
                            field,
                            span,
                        };
                    }
                    continue;
                }
                TokenKind::LBracket => {
                    let lhs_span = lhs.span();
                    self.advance();
                    let index = self.parse_expr(0)?;
                    // Check for range index: arr[start..end]
                    if *self.peek() == TokenKind::DotDot {
                        self.advance();
                        let end_expr = self.parse_expr(0)?;
                        let end = self.peek_span();
                        self.expect(&TokenKind::RBracket)?;
                        lhs = Expr::RangeIndex {
                            object: Box::new(lhs),
                            start: Box::new(index),
                            end: Box::new(end_expr),
                            span: lhs_span.merge(end),
                        };
                        continue;
                    }
                    let end = self.peek_span();
                    self.expect(&TokenKind::RBracket)?;
                    lhs = Expr::Index {
                        object: Box::new(lhs),
                        index: Box::new(index),
                        span: lhs_span.merge(end),
                    };
                    continue;
                }
                TokenKind::Question => {
                    let span = lhs.span().merge(self.advance().span);
                    lhs = Expr::Try {
                        operand: Box::new(lhs),
                        span,
                    };
                    continue;
                }
                TokenKind::As => {
                    self.advance();
                    let target_type = self.parse_type_annotation()?;
                    let span = lhs.span().merge(target_type.span());
                    lhs = Expr::Cast {
                        expr: Box::new(lhs),
                        target_type,
                        span,
                    };
                    continue;
                }
                _ => {}
            }

            // Infix operators
            if let Some((l_bp, r_bp)) = infix_bp(self.peek()) {
                if l_bp < min_bp {
                    break;
                }
                let op_token = self.advance().clone();
                let op = token_to_binop(&op_token.kind);
                let rhs = self.parse_expr(r_bp)?;
                let span = lhs.span().merge(rhs.span());
                lhs = Expr::BinOp {
                    op,
                    lhs: Box::new(lhs),
                    rhs: Box::new(rhs),
                    span,
                };
            } else {
                break;
            }
        }

        Ok(lhs)
    }

    fn parse_prefix(&mut self) -> Result<Expr, CompileError> {
        match self.peek().clone() {
            TokenKind::IntLit(n) => {
                let span = self.advance().span;
                Ok(Expr::Literal {
                    value: LitValue::Int(n),
                    span,
                })
            }
            TokenKind::FloatLit(f) => {
                let span = self.advance().span;
                Ok(Expr::Literal {
                    value: LitValue::Float(f),
                    span,
                })
            }
            TokenKind::BoolLit(b) => {
                let span = self.advance().span;
                Ok(Expr::Literal {
                    value: LitValue::Bool(b),
                    span,
                })
            }
            TokenKind::StringLit(s) => {
                let s = s.clone();
                let span = self.advance().span;
                Ok(Expr::Literal {
                    value: LitValue::Str(s),
                    span,
                })
            }
            TokenKind::FStringLit(raw) => {
                let raw = raw.clone();
                let span = self.advance().span;
                // Desugar f"text {expr} text" → "text" + to_str(expr) + "text"
                self.desugar_fstring(&raw, span)
            }
            TokenKind::Ident(name) => {
                let name = name.clone();
                let start = self.advance().span;

                // Enum variant: EnumName::Variant or EnumName::Variant(args)
                if *self.peek() == TokenKind::ColonColon {
                    self.advance();
                    let (variant, variant_span) = self.expect_ident()?;
                    // Check for payload args: EnumName::Variant(arg1, arg2)
                    let mut args = Vec::new();
                    let end_span = if *self.peek() == TokenKind::LParen {
                        self.advance();
                        while *self.peek() != TokenKind::RParen {
                            if !args.is_empty() {
                                self.expect(&TokenKind::Comma)?;
                            }
                            args.push(self.parse_expr(0)?);
                        }
                        let end = self.peek_span();
                        self.expect(&TokenKind::RParen)?;
                        end
                    } else {
                        variant_span
                    };
                    return Ok(Expr::EnumVariant {
                        enum_name: name,
                        variant,
                        args,
                        span: start.merge(end_span),
                    });
                }

                // Struct init: Name { field: expr, ... }
                if *self.peek() == TokenKind::LBrace && self.looks_like_struct_init() {
                    return self.parse_struct_init(name, start);
                }

                // Function call: name(args)
                if *self.peek() == TokenKind::LParen {
                    self.advance();
                    let mut args = Vec::new();
                    while *self.peek() != TokenKind::RParen {
                        if !args.is_empty() {
                            self.expect(&TokenKind::Comma)?;
                        }
                        args.push(self.parse_expr(0)?);
                    }
                    let end = self.peek_span();
                    self.expect(&TokenKind::RParen)?;
                    Ok(Expr::Call {
                        callee: name,
                        args,
                        span: start.merge(end),
                    })
                } else {
                    Ok(Expr::Ident { name, span: start })
                }
            }
            TokenKind::LParen => {
                let start = self.advance().span;
                let first = self.parse_expr(0)?;
                if *self.peek() == TokenKind::Comma {
                    // Tuple literal: (expr1, expr2, ...)
                    let mut elements = vec![first];
                    while *self.peek() == TokenKind::Comma {
                        self.advance();
                        if *self.peek() == TokenKind::RParen {
                            break;
                        }
                        elements.push(self.parse_expr(0)?);
                    }
                    let end = self.peek_span();
                    self.expect(&TokenKind::RParen)?;
                    Ok(Expr::TupleLit {
                        elements,
                        span: start.merge(end),
                    })
                } else {
                    // Parenthesized expression
                    self.expect(&TokenKind::RParen)?;
                    Ok(first)
                }
            }
            TokenKind::LBracket => {
                let start = self.advance().span;
                let mut elements = Vec::new();
                while *self.peek() != TokenKind::RBracket {
                    if !elements.is_empty() {
                        self.expect(&TokenKind::Comma)?;
                    }
                    elements.push(self.parse_expr(0)?);
                }
                let end = self.peek_span();
                self.expect(&TokenKind::RBracket)?;
                Ok(Expr::ArrayLit {
                    elements,
                    span: start.merge(end),
                })
            }
            TokenKind::Ampersand => {
                let start = self.advance().span;
                let operand = self.parse_expr(11)?;
                let span = start.merge(operand.span());
                Ok(Expr::AddrOf {
                    operand: Box::new(operand),
                    span,
                })
            }
            TokenKind::Star => {
                let start = self.advance().span;
                let operand = self.parse_expr(11)?;
                let span = start.merge(operand.span());
                Ok(Expr::Deref {
                    operand: Box::new(operand),
                    span,
                })
            }
            TokenKind::If => self.parse_if_expr(),
            TokenKind::Match => self.parse_match_expr(),
            // Lambda: |params| -> RetType { body } or |params| -> RetType = expr;
            TokenKind::Pipe => {
                let start = self.advance().span;
                let mut params = Vec::new();
                // Parse until closing |
                while *self.peek() != TokenKind::Pipe {
                    if !params.is_empty() {
                        self.expect(&TokenKind::Comma)?;
                    }
                    let param_start = self.peek_span();
                    let (param_name, _) = self.expect_ident()?;
                    self.expect(&TokenKind::Colon)?;
                    let ty = self.parse_type_annotation()?;
                    let param_span = param_start.merge(ty.span());
                    params.push(Param {
                        name: param_name,
                        ty,
                        span: param_span,
                    });
                }
                self.expect(&TokenKind::Pipe)?; // closing |
                self.expect(&TokenKind::Arrow)?;
                let return_type = self.parse_type_annotation()?;
                let body = self.parse_block_expr()?;
                let span = start.merge(body.span());
                Ok(Expr::Lambda {
                    params,
                    return_type,
                    body: Box::new(body),
                    span,
                })
            }
            // Also handle || (Or token) as zero-param lambda
            TokenKind::Or => {
                let start = self.advance().span;
                self.expect(&TokenKind::Arrow)?;
                let return_type = self.parse_type_annotation()?;
                let body = self.parse_block_expr()?;
                let span = start.merge(body.span());
                Ok(Expr::Lambda {
                    params: Vec::new(),
                    return_type,
                    body: Box::new(body),
                    span,
                })
            }
            TokenKind::Minus | TokenKind::Not | TokenKind::Tilde => {
                let bp = prefix_bp(self.peek()).unwrap();
                let op_token = self.advance().clone();
                let op = match op_token.kind {
                    TokenKind::Minus => UnaryOp::Neg,
                    TokenKind::Not => UnaryOp::Not,
                    TokenKind::Tilde => UnaryOp::BitNot,
                    _ => unreachable!(),
                };
                let operand = self.parse_expr(bp)?;
                let span = op_token.span.merge(operand.span());
                Ok(Expr::UnaryOp {
                    op,
                    operand: Box::new(operand),
                    span,
                })
            }
            _ => Err(CompileError::syntax(
                format!("expected expression, found {:?}", self.peek()),
                self.peek_span(),
            )),
        }
    }

    fn desugar_fstring(&self, raw: &str, span: Span) -> Result<Expr, CompileError> {
        // Parse f-string: split on { and }
        // "hello {name}, age {age}" → ["hello ", name, ", age ", age]
        let mut parts: Vec<Expr> = Vec::new();
        let mut text = String::new();
        let mut in_expr = false;
        let mut expr_str = String::new();

        for ch in raw.chars() {
            if ch == '{' && !in_expr {
                if !text.is_empty() {
                    parts.push(Expr::Literal {
                        value: LitValue::Str(text.clone()),
                        span,
                    });
                    text.clear();
                }
                in_expr = true;
                expr_str.clear();
            } else if ch == '}' && in_expr {
                in_expr = false;
                let trimmed = expr_str.trim();
                if trimmed.is_empty() {
                    return Err(CompileError::syntax("empty expression in f-string", span));
                }
                let inner_expr = parse_expression_from_source(trimmed, span)?;
                parts.push(Expr::Call {
                    callee: "to_str".to_string(),
                    args: vec![inner_expr],
                    span,
                });
                expr_str.clear();
            } else if in_expr {
                expr_str.push(ch);
            } else {
                text.push(ch);
            }
        }
        if !text.is_empty() {
            parts.push(Expr::Literal {
                value: LitValue::Str(text),
                span,
            });
        }

        // Concatenate all parts with +
        if parts.is_empty() {
            return Ok(Expr::Literal {
                value: LitValue::Str(String::new()),
                span,
            });
        }
        let mut result = parts.remove(0);
        for part in parts {
            result = Expr::BinOp {
                op: BinOp::Add,
                lhs: Box::new(result),
                rhs: Box::new(part),
                span,
            };
        }
        Ok(result)
    }

    fn parse_match_expr(&mut self) -> Result<Expr, CompileError> {
        let start = self.peek_span();
        self.expect(&TokenKind::Match)?;
        let subject = Box::new(self.parse_expr(0)?);
        self.expect(&TokenKind::LBrace)?;

        let mut arms = Vec::new();
        while *self.peek() != TokenKind::RBrace && *self.peek() != TokenKind::Eof {
            let pattern = self.parse_pattern()?;
            self.expect(&TokenKind::FatArrow)?;
            let body = if *self.peek() == TokenKind::LBrace {
                self.parse_block_expr()?
            } else {
                self.parse_expr(0)?
            };
            arms.push(MatchArm { pattern, body });
            if *self.peek() == TokenKind::Comma {
                self.advance();
            }
        }

        let end = self.peek_span();
        self.expect(&TokenKind::RBrace)?;
        Ok(Expr::Match {
            subject,
            arms,
            span: start.merge(end),
        })
    }

    fn parse_pattern(&mut self) -> Result<Pattern, CompileError> {
        match self.peek().clone() {
            TokenKind::Underscore => {
                let span = self.advance().span;
                Ok(Pattern::Wildcard(span))
            }
            TokenKind::IntLit(n) => {
                let span = self.advance().span;
                Ok(Pattern::IntLit(n, span))
            }
            TokenKind::Ident(name) => {
                let name = name.clone();
                let start = self.advance().span;
                self.expect(&TokenKind::ColonColon)?;
                let (variant, variant_span) = self.expect_ident()?;
                // Check for bindings: Pattern::Variant(a, b)
                let mut bindings = Vec::new();
                let end_span = if *self.peek() == TokenKind::LParen {
                    self.advance();
                    while *self.peek() != TokenKind::RParen {
                        if !bindings.is_empty() {
                            self.expect(&TokenKind::Comma)?;
                        }
                        let (binding_name, _) = self.expect_ident()?;
                        bindings.push(binding_name);
                    }
                    let end = self.peek_span();
                    self.expect(&TokenKind::RParen)?;
                    end
                } else {
                    variant_span
                };
                Ok(Pattern::EnumVariant {
                    enum_name: name,
                    variant,
                    bindings,
                    span: start.merge(end_span),
                })
            }
            _ => Err(CompileError::syntax(
                format!("expected pattern, found {:?}", self.peek()),
                self.peek_span(),
            )),
        }
    }

    fn is_tuple_destructure(&self) -> bool {
        // Look for pattern: ( ident , → likely tuple destructuring
        if *self.peek() == TokenKind::LParen && self.pos + 3 < self.tokens.len() {
            matches!(
                (
                    &self.tokens[self.pos + 1].kind,
                    &self.tokens[self.pos + 2].kind
                ),
                (TokenKind::Ident(_), TokenKind::Comma)
            )
        } else {
            false
        }
    }

    fn parse_tuple_destructure(&mut self) -> Result<Stmt, CompileError> {
        let start = self.peek_span();
        self.expect(&TokenKind::LParen)?;

        let mut names = Vec::new();
        while *self.peek() != TokenKind::RParen {
            if !names.is_empty() {
                self.expect(&TokenKind::Comma)?;
            }
            let (name, _) = self.expect_ident()?;
            names.push(name);
        }
        self.expect(&TokenKind::RParen)?;

        let mutability = match self.peek() {
            TokenKind::ColonAssign => {
                self.advance();
                Mutability::Immutable
            }
            TokenKind::ColonTildeAssign => {
                self.advance();
                Mutability::Mutable
            }
            _ => {
                return Err(CompileError::syntax(
                    format!(
                        "expected ':=' or ':~=' after tuple destructure, found {:?}",
                        self.peek()
                    ),
                    self.peek_span(),
                ));
            }
        };

        let init = self.parse_expr(0)?;
        let end = self.peek_span();
        self.expect(&TokenKind::Semi)?;

        Ok(Stmt::TupleDestructure {
            names,
            mutability,
            init,
            span: start.merge(end),
        })
    }

    fn looks_like_struct_init(&self) -> bool {
        if self.pos + 2 < self.tokens.len() {
            matches!(
                (
                    &self.tokens[self.pos + 1].kind,
                    &self.tokens[self.pos + 2].kind
                ),
                (TokenKind::Ident(_), TokenKind::Colon)
            )
        } else {
            false
        }
    }

    fn parse_struct_init(&mut self, name: String, start: Span) -> Result<Expr, CompileError> {
        self.expect(&TokenKind::LBrace)?;
        let mut fields = Vec::new();
        while *self.peek() != TokenKind::RBrace {
            if !fields.is_empty() {
                self.expect(&TokenKind::Comma)?;
                if *self.peek() == TokenKind::RBrace {
                    break;
                }
            }
            let (field_name, _) = self.expect_ident()?;
            self.expect(&TokenKind::Colon)?;
            let value = self.parse_expr(0)?;
            fields.push((field_name, value));
        }
        let end = self.peek_span();
        self.expect(&TokenKind::RBrace)?;
        Ok(Expr::StructInit {
            name,
            fields,
            span: start.merge(end),
        })
    }
}

fn token_to_binop(kind: &TokenKind) -> BinOp {
    match kind {
        TokenKind::Plus => BinOp::Add,
        TokenKind::Minus => BinOp::Sub,
        TokenKind::Star => BinOp::Mul,
        TokenKind::Slash => BinOp::Div,
        TokenKind::Percent => BinOp::Mod,
        TokenKind::Eq => BinOp::Eq,
        TokenKind::Ne => BinOp::Ne,
        TokenKind::Lt => BinOp::Lt,
        TokenKind::Gt => BinOp::Gt,
        TokenKind::Le => BinOp::Le,
        TokenKind::Ge => BinOp::Ge,
        TokenKind::And => BinOp::And,
        TokenKind::Or => BinOp::Or,
        TokenKind::Ampersand => BinOp::BitAnd,
        TokenKind::Pipe => BinOp::BitOr,
        TokenKind::Caret => BinOp::BitXor,
        TokenKind::LtLt => BinOp::Shl,
        TokenKind::GtGt => BinOp::Shr,
        _ => unreachable!("not a binary operator: {:?}", kind),
    }
}

fn compound_to_binop(kind: &TokenKind) -> Option<BinOp> {
    match kind {
        TokenKind::PlusAssign => Some(BinOp::Add),
        TokenKind::MinusAssign => Some(BinOp::Sub),
        TokenKind::StarAssign => Some(BinOp::Mul),
        TokenKind::SlashAssign => Some(BinOp::Div),
        TokenKind::PercentAssign => Some(BinOp::Mod),
        TokenKind::AmpAssign => Some(BinOp::BitAnd),
        TokenKind::PipeAssign => Some(BinOp::BitOr),
        TokenKind::CaretAssign => Some(BinOp::BitXor),
        TokenKind::LtLtAssign => Some(BinOp::Shl),
        TokenKind::GtGtAssign => Some(BinOp::Shr),
        _ => None,
    }
}

pub fn parse(tokens: Vec<Token>) -> Result<Program, Vec<CompileError>> {
    let mut parser = Parser::new(tokens);
    parser.parse_program()
}
