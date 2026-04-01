pub mod ast;
pub mod precedence;

use crate::common::{CompileError, Span};
use crate::lexer::token::{Token, TokenKind};
use ast::*;
use precedence::{infix_bp, prefix_bp};

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

        // Array type: [N]T
        if *self.peek() == TokenKind::LBracket {
            self.advance();
            let size = match self.peek().clone() {
                TokenKind::IntLit(n) => {
                    self.advance();
                    n as usize
                }
                _ => {
                    return Err(CompileError::syntax(
                        "expected array size (integer literal)".to_string(),
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

        // Named type
        let (name, span) = self.expect_ident()?;
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
                        TokenKind::Fn | TokenKind::Struct | TokenKind::Eof
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
        match self.peek() {
            TokenKind::Fn => self.parse_function(),
            TokenKind::Struct => self.parse_struct_def(),
            _ => Err(CompileError::syntax(
                format!("expected 'fn' or 'struct', found {:?}", self.peek()),
                self.peek_span(),
            )),
        }
    }

    fn parse_struct_def(&mut self) -> Result<Item, CompileError> {
        let start = self.peek_span();
        self.expect(&TokenKind::Struct)?;
        let (name, _) = self.expect_ident()?;
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
            fields,
            span: start.merge(end),
        })
    }

    fn parse_function(&mut self) -> Result<Item, CompileError> {
        let start_span = self.peek_span();
        self.expect(&TokenKind::Fn)?;
        let (name, _) = self.expect_ident()?;
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
                matches!(
                    self.tokens[self.pos + 1].kind,
                    TokenKind::Colon
                        | TokenKind::ColonTilde
                        | TokenKind::ColonColon
                        | TokenKind::ColonAssign
                        | TokenKind::ColonTildeAssign
                        | TokenKind::Assign
                )
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
        let range_start = self.parse_expr(0)?;

        let inclusive = match self.peek() {
            TokenKind::DotDot => {
                self.advance();
                false
            }
            TokenKind::DotDotEq => {
                self.advance();
                true
            }
            _ => {
                return Err(CompileError::syntax(
                    format!(
                        "expected '..' or '..=' in for range, found {:?}",
                        self.peek()
                    ),
                    self.peek_span(),
                ));
            }
        };

        let range_end = self.parse_expr(0)?;
        let body = self.parse_block_expr()?;
        let end = body.span();

        Ok(Stmt::ForRange {
            var,
            start: range_start,
            end: range_end,
            inclusive,
            body,
            span: start.merge(end),
        })
    }

    fn parse_if_expr(&mut self) -> Result<Expr, CompileError> {
        let start = self.peek_span();
        self.expect(&TokenKind::If)?;
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

    fn parse_expr(&mut self, min_bp: u8) -> Result<Expr, CompileError> {
        let mut lhs = self.parse_prefix()?;

        loop {
            if *self.peek() == TokenKind::Eof {
                break;
            }

            // Postfix: . (field access / method call), [ (index)
            match self.peek() {
                TokenKind::Dot => {
                    self.advance();
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
                    let end = self.peek_span();
                    self.expect(&TokenKind::RBracket)?;
                    lhs = Expr::Index {
                        object: Box::new(lhs),
                        index: Box::new(index),
                        span: lhs_span.merge(end),
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
            TokenKind::Ident(name) => {
                let name = name.clone();
                let start = self.advance().span;

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
                self.advance();
                let expr = self.parse_expr(0)?;
                self.expect(&TokenKind::RParen)?;
                Ok(expr)
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
