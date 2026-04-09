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
    errors: Vec<CompileError>,
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Self {
            tokens,
            pos: 0,
            errors: Vec::new(),
        }
    }

    /// Skip tokens until we find a synchronization point for statement recovery.
    fn synchronize_stmt(&mut self) {
        loop {
            match self.peek() {
                TokenKind::Semi => {
                    self.advance(); // consume the semicolon
                    return;
                }
                TokenKind::RBrace | TokenKind::Eof => return,
                // Statement-starting tokens — don't consume, let the caller retry
                TokenKind::Return
                | TokenKind::While
                | TokenKind::For
                | TokenKind::If
                | TokenKind::Break
                | TokenKind::Continue
                | TokenKind::Defer
                | TokenKind::Loop
                | TokenKind::Select
                | TokenKind::Var => return,
                _ => {
                    self.advance();
                }
            }
        }
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

        // Optional type: ?T
        if *self.peek() == TokenKind::Question {
            self.advance();
            let inner = self.parse_type_annotation()?;
            let span = start.merge(inner.span());
            return Ok(TypeAnnotation::Optional {
                inner: Box::new(inner),
                span,
            });
        }

        // Dynamic trait object: dyn Trait
        if *self.peek() == TokenKind::Dyn {
            self.advance();
            let (trait_name, end) = self.expect_ident()?;
            return Ok(TypeAnnotation::DynTrait {
                trait_name,
                span: start.merge(end),
            });
        }

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
            // Check for slice: []T (no size expression)
            if *self.peek() == TokenKind::RBracket {
                self.advance();
                let element_type = self.parse_type_annotation()?;
                let span = start.merge(element_type.span());
                return Ok(TypeAnnotation::Slice {
                    element_type: Box::new(element_type),
                    span,
                });
            }

            let size_expr = self.parse_expr(0)?;
            self.expect(&TokenKind::RBracket)?;
            let element_type = self.parse_type_annotation()?;
            let span = start.merge(element_type.span());
            return Ok(TypeAnnotation::Array {
                size: Box::new(size_expr),
                element_type: Box::new(element_type),
                span,
            });
        }

        let (name, name_span) = self.expect_ident()?;

        // Generic type: Vec<T>
        if *self.peek() == TokenKind::Lt {
            self.advance();
            let mut params = Vec::new();
            while *self.peek() != TokenKind::Gt && *self.peek() != TokenKind::Eof {
                params.push(self.parse_type_annotation()?);
                if *self.peek() == TokenKind::Comma {
                    self.advance();
                }
            }
            let end = self.expect(&TokenKind::Gt)?;
            return Ok(TypeAnnotation::Generic {
                base: name,
                params,
                span: start.merge(end),
            });
        }

        Ok(TypeAnnotation::Named {
            name,
            span: name_span,
        })
    }

    fn parse_param(&mut self) -> Result<Param, CompileError> {
        let (name, start) = self.expect_ident()?;
        self.expect(&TokenKind::Colon)?;
        let ty = self.parse_type_annotation()?;
        let span = start.merge(ty.span());
        Ok(Param { name, ty, span })
    }

    fn parse_param_list(&mut self) -> Result<Vec<Param>, CompileError> {
        self.expect(&TokenKind::LParen)?;
        let mut params = Vec::new();
        if *self.peek() != TokenKind::RParen {
            loop {
                params.push(self.parse_param()?);
                if *self.peek() == TokenKind::Comma {
                    self.advance();
                } else {
                    break;
                }
            }
        }
        self.expect(&TokenKind::RParen)?;
        Ok(params)
    }

    fn parse_type_param_list(&mut self) -> Result<Vec<TypeParam>, CompileError> {
        let mut params = Vec::new();
        if *self.peek() == TokenKind::Lt {
            self.advance();
            while *self.peek() != TokenKind::Gt && *self.peek() != TokenKind::Eof {
                let (name, span) = self.expect_ident()?;
                let bounds = self.parse_type_param_bounds()?;
                params.push(TypeParam { name, bounds, span });
                if *self.peek() == TokenKind::Comma {
                    self.advance();
                }
            }
            self.expect(&TokenKind::Gt)?;
        }
        Ok(params)
    }

    fn parse_function_def(&mut self, is_async: bool) -> Result<Item, CompileError> {
        let start = self.expect(&TokenKind::Fn)?.start;
        let (name, _) = self.expect_ident()?;
        let type_params = self.parse_type_param_list()?;
        let params = self.parse_param_list()?;
        let return_type = if *self.peek() == TokenKind::Arrow {
            self.advance();
            self.parse_type_annotation()?
        } else {
            TypeAnnotation::Named {
                name: "unit".to_string(),
                span: self.peek_span(),
            }
        };
        let body = self.parse_expr(0)?;
        let span = start.merge(body.span());
        Ok(Item::FunctionDef {
            name,
            is_async,
            type_params,
            params,
            return_type,
            body,
            span,
        })
    }

    fn parse_struct_def(&mut self) -> Result<Item, CompileError> {
        let start = self.expect(&TokenKind::Struct)?.start;
        let (name, _) = self.expect_ident()?;
        let type_params = self.parse_type_param_list()?;
        self.expect(&TokenKind::LBrace)?;
        let mut fields = Vec::new();
        while *self.peek() != TokenKind::RBrace && *self.peek() != TokenKind::Eof {
            let (field_name, _) = self.expect_ident()?;
            self.expect(&TokenKind::Colon)?;
            let ty = self.parse_type_annotation()?;
            fields.push((field_name, ty));
            if *self.peek() == TokenKind::Comma {
                self.advance();
            }
        }
        let end = self.expect(&TokenKind::RBrace)?;
        Ok(Item::StructDef {
            name,
            type_params,
            fields,
            span: start.merge(end),
        })
    }

    fn parse_enum_variant(&mut self) -> Result<EnumVariantDef, CompileError> {
        let (name, start) = self.expect_ident()?;
        let mut payload = Vec::new();
        let mut end = start;
        if *self.peek() == TokenKind::LParen {
            self.advance();
            while *self.peek() != TokenKind::RParen && *self.peek() != TokenKind::Eof {
                payload.push(self.parse_type_annotation()?);
                if *self.peek() == TokenKind::Comma {
                    self.advance();
                }
            }
            end = self.expect(&TokenKind::RParen)?;
        }
        Ok(EnumVariantDef {
            name,
            payload,
            span: start.merge(end),
        })
    }

    fn parse_enum_def(&mut self) -> Result<Item, CompileError> {
        let start = self.expect(&TokenKind::Enum)?.start;
        let (name, _) = self.expect_ident()?;
        let type_params = self.parse_type_param_list()?;
        self.expect(&TokenKind::LBrace)?;
        let mut variants = Vec::new();
        while *self.peek() != TokenKind::RBrace && *self.peek() != TokenKind::Eof {
            variants.push(self.parse_enum_variant()?);
            if *self.peek() == TokenKind::Comma {
                self.advance();
            }
        }
        let end = self.expect(&TokenKind::RBrace)?;
        Ok(Item::EnumDef {
            name,
            type_params,
            variants,
            span: start.merge(end),
        })
    }

    fn parse_use(&mut self) -> Result<Item, CompileError> {
        let start = self.expect(&TokenKind::Use)?.start;
        // Simplified path parsing for now
        let (path, end) = self.expect_ident()?;
        self.expect(&TokenKind::Semi)?;
        Ok(Item::Use {
            path,
            span: start.merge(end),
        })
    }

    fn parse_extern_fn_decl(&mut self) -> Result<ExternFnDecl, CompileError> {
        let start = self.expect(&TokenKind::Fn)?.start;
        let (name, _) = self.expect_ident()?;
        self.expect(&TokenKind::LParen)?;
        let mut params = Vec::new();
        let mut variadic = false;
        if *self.peek() != TokenKind::RParen {
            loop {
                if *self.peek() == TokenKind::DotDotDot {
                    self.advance();
                    variadic = true;
                    break;
                }
                params.push(self.parse_param()?);
                if *self.peek() == TokenKind::Comma {
                    self.advance();
                } else {
                    break;
                }
            }
        }
        self.expect(&TokenKind::RParen)?;
        let return_type = if *self.peek() == TokenKind::Arrow {
            self.advance();
            self.parse_type_annotation()?
        } else {
            TypeAnnotation::Named {
                name: "unit".to_string(),
                span: self.peek_span(),
            }
        };
        let end = self.expect(&TokenKind::Semi)?;
        Ok(ExternFnDecl {
            name,
            params,
            return_type,
            variadic,
            span: start.merge(end),
        })
    }

    fn parse_extern_block(&mut self) -> Result<Item, CompileError> {
        let start = self.expect(&TokenKind::Extern)?.start;

        if let TokenKind::String(lang) = self.peek().clone() {
            self.advance();
            self.expect(&TokenKind::LBrace)?;
            let mut functions = Vec::new();
            while *self.peek() != TokenKind::RBrace && *self.peek() != TokenKind::Eof {
                functions.push(self.parse_extern_fn_decl()?);
            }
            let end = self.expect(&TokenKind::RBrace)?;
            return Ok(Item::GpuExternBlock {
                lang,
                functions,
                span: start.merge(end),
            });
        }

        self.expect(&TokenKind::LBrace)?;
        let mut functions = Vec::new();
        while *self.peek() != TokenKind::RBrace && *self.peek() != TokenKind::Eof {
            functions.push(self.parse_extern_fn_decl()?);
        }
        let end = self.expect(&TokenKind::RBrace)?;
        Ok(Item::ExternBlock {
            functions,
            span: start.merge(end),
        })
    }

    fn parse_item(&mut self) -> Result<Item, CompileError> {
        match self.peek() {
            TokenKind::Fn => self.parse_function_def(false),
            TokenKind::Async => {
                self.advance();
                self.parse_function_def(true)
            }
            TokenKind::Struct => self.parse_struct_def(),
            TokenKind::Enum => self.parse_enum_def(),
            TokenKind::Use => self.parse_use(),
            TokenKind::Extern => self.parse_extern_block(),
            _ => Err(CompileError::syntax(
                format!("unexpected token {:?} at top level", self.peek()),
                self.peek_span(),
            )),
        }
    }

    pub fn parse_program(&mut self) -> Result<Program, Vec<CompileError>> {
        let mut items = Vec::new();
        while *self.peek() != TokenKind::Eof {
            match self.parse_item() {
                Ok(item) => items.push(item),
                Err(e) => {
                    self.errors.push(e);
                    // Add recovery logic here if needed
                    while *self.peek() != TokenKind::Eof && *self.peek() != TokenKind::Fn {
                        self.advance();
                    }
                }
            }
        }
        if self.errors.is_empty() {
            Ok(Program { items })
        } else {
            Err(self.errors.clone())
        }
    }

    // ... expression parsing methods (parse_expr, parse_stmt, etc.) would go here
    // This is a simplified skeleton focusing on top-level items.
    // A full implementation would be much larger.

    pub fn parse_expr(&mut self, min_bp: u8) -> Result<Expr, CompileError> {
        let mut lhs = self.parse_prefix()?;

        loop {
            let op_token = self.peek().clone();
            if op_token == TokenKind::Eof || op_token == TokenKind::RParen || op_token == TokenKind::RBrace || op_token == TokenKind::RBracket || op_token == TokenKind::Comma || op_token == TokenKind::Semi {
                break;
            }

            if let Some((l_bp, r_bp)) = infix_bp(&op_token) {
                if l_bp < min_bp {
                    break;
                }
                self.advance();
                let rhs = self.parse_expr(r_bp)?;
                let span = lhs.span().merge(rhs.span());
                lhs = self.parse_infix(lhs, op_token, rhs, span)?;
                continue;
            }
            break;
        }

        Ok(lhs)
    }

    fn parse_prefix(&mut self) -> Result<Expr, CompileError> {
        let token = self.advance().clone();
        let (op, r_bp) = match prefix_bp(&token.kind) {
            Some(op) => op,
            None => {
                // If not a prefix operator, it must be a primary expression
                return self.parse_primary(token);
            }
        };

        let operand = self.parse_expr(r_bp)?;
        let span = token.span.merge(operand.span());
        Ok(Expr::UnaryOp {
            op,
            operand: Box::new(operand),
            span,
        })
    }

    fn parse_infix(&mut self, lhs: Expr, op_token: TokenKind, rhs: Expr, span: Span) -> Result<Expr, CompileError> {
        let op = match op_token {
            TokenKind::Plus => BinOp::Add,
            TokenKind::Minus => BinOp::Sub,
            TokenKind::Star => BinOp::Mul,
            TokenKind::Slash => BinOp::Div,
            TokenKind::Percent => BinOp::Mod,
            TokenKind::EqEq => BinOp::Eq,
            TokenKind::NotEq => BinOp::NotEq,
            TokenKind::Lt => BinOp::Lt,
            TokenKind::Gt => BinOp::Gt,
            TokenKind::LtEq => BinOp::LtEq,
            TokenKind::GtEq => BinOp::GtEq,
            TokenKind::AndAnd => BinOp::And,
            TokenKind::OrOr => BinOp::Or,
            TokenKind::And => BinOp::BitwiseAnd,
            TokenKind::Or => BinOp::BitwiseOr,
            TokenKind::Caret => BinOp::BitwiseXor,
            TokenKind::Shl => BinOp::Shl,
            TokenKind::Shr => BinOp::Shr,
            TokenKind::DotDot => BinOp::Range,
            TokenKind::DotDotEq => BinOp::RangeInclusive,
            TokenKind::LtMinus => BinOp::Send,
            _ => return Err(CompileError::syntax(format!("unexpected infix operator {:?}", op_token), span)),
        };
        Ok(Expr::BinOp { op, lhs: Box::new(lhs), rhs: Box::new(rhs), span })
    }


    fn parse_primary(&mut self, token: Token) -> Result<Expr, CompileError> {
        match token.kind {
            TokenKind::Int(val) => Ok(Expr::Literal { value: LitValue::Int(val), span: token.span }),
            TokenKind::Float(val) => Ok(Expr::Literal { value: LitValue::Float(val), span: token.span }),
            TokenKind::String(val) => Ok(Expr::Literal { value: LitValue::String(val), span: token.span }),
            TokenKind::Char(val) => Ok(Expr::Literal { value: LitValue::Char(val), span: token.span }),
            TokenKind::True => Ok(Expr::Literal { value: LitValue::Bool(true), span: token.span }),
            TokenKind::False => Ok(Expr::Literal { value: LitValue::Bool(false), span: token.span }),
            TokenKind::Ident(name) => Ok(Expr::Ident { name, span: token.span }),
            TokenKind::LParen => {
                let expr = self.parse_expr(0)?;
                self.expect(&TokenKind::RParen)?;
                Ok(expr)
            }
            TokenKind::LBrace => self.parse_block_expr(token.span),
            _ => Err(CompileError::syntax(format!("unexpected token in expression: {:?}", token.kind), token.span)),
        }
    }

    fn parse_block_expr(&mut self, start: Span) -> Result<Expr, CompileError> {
        let mut stmts = Vec::new();
        let mut expr = None;

        while *self.peek() != TokenKind::RBrace && *self.peek() != TokenKind::Eof {
            let stmt = self.parse_stmt()?;
            // Check if the next token is a semicolon. If not, this statement
            // is the trailing expression of the block.
            if *self.peek() != TokenKind::Semi && *self.peek() != TokenKind::RBrace {
                 if let Stmt::ExprStmt { expr: e, .. } = stmt {
                    expr = Some(Box::new(e));
                 } else {
                    // This is an error case, a statement that's not an expression
                    // is not followed by a semicolon.
                    self.errors.push(CompileError::syntax(
                        "expected semicolon after statement".to_string(),
                        self.peek_span(),
                    ));
                    stmts.push(stmt); // still push it to attempt to continue parsing
                 }
                 break; // exit loop, as we've found the trailing expression
            }

            stmts.push(stmt);

            // Consume optional semicolon
            if *self.peek() == TokenKind::Semi {
                self.advance();
            }
        }

        let end = self.expect(&TokenKind::RBrace)?;
        Ok(Expr::Block {
            stmts,
            expr,
            span: start.merge(end),
        })
    }

    fn parse_stmt(&mut self) -> Result<Stmt, CompileError> {
        match self.peek() {
            TokenKind::Var | TokenKind::Const => self.parse_var_decl(),
            TokenKind::Return => self.parse_return(),
            _ => {
                let expr = self.parse_expr(0)?;
                let span = expr.span();
                Ok(Stmt::ExprStmt { expr, span })
            }
        }
    }

    fn parse_var_decl(&mut self) -> Result<Stmt, CompileError> {
        let is_const = *self.peek() == TokenKind::Const;
        let start = self.advance().span;

        let (name, _) = self.expect_ident()?;
        let mut ty = None;
        if *self.peek() == TokenKind::Colon {
            self.advance();
            ty = Some(self.parse_type_annotation()?);
        }

        self.expect(&TokenKind::Eq)?;
        let init = self.parse_expr(0)?;
        let span = start.merge(init.span());

        if is_const {
            Ok(Stmt::ConstDecl { name, ty, value: init, span })
        } else {
            Ok(Stmt::VarDecl { name, mutability: Mutability::Immutable, ty, init, span })
        }
    }

    fn parse_return(&mut self) -> Result<Stmt, CompileError> {
        let start = self.expect(&TokenKind::Return)?;
        let value = if *self.peek() == TokenKind::Semi {
            None
        } else {
            Some(self.parse_expr(0)?)
        };
        let end_span = value.as_ref().map(|v| v.span()).unwrap_or(start);
        Ok(Stmt::Return { value, span: start.merge(end_span) })
    }
}

impl Expr {
    pub fn span(&self) -> Span {
        match self {
            Expr::Literal { span, .. } => *span,
            Expr::Ident { span, .. } => *span,
            Expr::BinOp { span, .. } => *span,
            Expr::UnaryOp { span, .. } => *span,
            Expr::Call { span, .. } => *span,
            Expr::If { span, .. } => *span,
            Expr::Block { span, .. } => *span,
            Expr::Array { span, .. } => *span,
            Expr::Index { span, .. } => *span,
            Expr::Tuple { span, .. } => *span,
            Expr::StructInit { span, .. } => *span,
            Expr::FieldAccess { span, .. } => *span,
            Expr::MethodCall { span, .. } => *span,
            Expr::EnumInit { span, .. } => *span,
            Expr::Match { span, .. } => *span,
            Expr::Lambda { span, .. } => *span,
            Expr::Await { span, .. } => *span,
            Expr::FString { span, .. } => *span,
            Expr::Cast { span, .. } => *span,
            Expr::Spawn { span, .. } => *span,
            Expr::AddrOf { span, .. } => *span,
            Expr::Deref { span, .. } => *span,
        }
    }
}
