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
        let (name, span) = self.expect_ident()?;
        Ok(TypeAnnotation { name, span })
    }

    fn parse_program(&mut self) -> Result<Program, Vec<CompileError>> {
        let mut items = Vec::new();
        let mut errors = Vec::new();

        while *self.peek() != TokenKind::Eof {
            match self.parse_item() {
                Ok(item) => items.push(item),
                Err(e) => {
                    errors.push(e);
                    // Skip to next fn or EOF
                    while *self.peek() != TokenKind::Fn && *self.peek() != TokenKind::Eof {
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
            _ => Err(CompileError::syntax(
                format!("expected 'fn', found {:?}", self.peek()),
                self.peek_span(),
            )),
        }
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
            let param_span = param_start.merge(ty.span);
            params.push(Param {
                name: param_name,
                ty,
                span: param_span,
            });
        }
        self.expect(&TokenKind::RParen)?;

        self.expect(&TokenKind::Arrow)?;
        let return_type = self.parse_type_annotation()?;

        // Support both block body and expression body (= expr;)
        let body = if *self.peek() == TokenKind::Assign {
            self.advance(); // consume '='
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
            // Try to parse a statement
            match self.peek() {
                TokenKind::Return => {
                    stmts.push(self.parse_return_stmt()?);
                }
                TokenKind::While => {
                    stmts.push(self.parse_while_stmt()?);
                }
                TokenKind::Ident(_) => {
                    // Could be: var decl (name : type = expr), const decl (name :: type = expr),
                    // assignment (name = expr), or expression statement
                    if self.is_var_decl_or_assign() {
                        stmts.push(self.parse_var_decl_or_assign()?);
                    } else {
                        let expr = self.parse_expr(0)?;
                        if *self.peek() == TokenKind::Semi {
                            let span = expr.span();
                            self.advance();
                            stmts.push(Stmt::ExprStmt { expr, span });
                        } else if *self.peek() == TokenKind::RBrace {
                            tail_expr = Some(Box::new(expr));
                        } else {
                            let span = expr.span();
                            stmts.push(Stmt::ExprStmt { expr, span });
                        }
                    }
                }
                TokenKind::If => {
                    let expr = self.parse_if_expr()?;
                    if *self.peek() == TokenKind::Semi {
                        let span = expr.span();
                        self.advance();
                        stmts.push(Stmt::ExprStmt { expr, span });
                    } else if *self.peek() == TokenKind::RBrace {
                        tail_expr = Some(Box::new(expr));
                    } else {
                        let span = expr.span();
                        stmts.push(Stmt::ExprStmt { expr, span });
                    }
                }
                _ => {
                    let expr = self.parse_expr(0)?;
                    if *self.peek() == TokenKind::Semi {
                        let span = expr.span();
                        self.advance();
                        stmts.push(Stmt::ExprStmt { expr, span });
                    } else if *self.peek() == TokenKind::RBrace {
                        tail_expr = Some(Box::new(expr));
                    } else {
                        return Err(CompileError::syntax(
                            format!("expected ';' or '}}', found {:?}", self.peek()),
                            self.peek_span(),
                        ));
                    }
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

    fn is_var_decl_or_assign(&self) -> bool {
        // Look ahead: ident followed by : or :~ or :: or =
        if let TokenKind::Ident(_) = &self.tokens[self.pos].kind {
            if self.pos + 1 < self.tokens.len() {
                matches!(
                    self.tokens[self.pos + 1].kind,
                    TokenKind::Colon
                        | TokenKind::ColonTilde
                        | TokenKind::ColonColon
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

        match self.peek() {
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
            TokenKind::Assign => {
                self.advance();
                let value = self.parse_expr(0)?;
                let end = self.peek_span();
                self.expect(&TokenKind::Semi)?;
                Ok(Stmt::Assign {
                    target: name,
                    value,
                    span: start.merge(end),
                })
            }
            _ => Err(CompileError::syntax(
                format!(
                    "expected ':', ':~', '::', or '=' after identifier, found {:?}",
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
                let n = n;
                let span = self.advance().span;
                Ok(Expr::Literal {
                    value: LitValue::Int(n),
                    span,
                })
            }
            TokenKind::FloatLit(f) => {
                let f = f;
                let span = self.advance().span;
                Ok(Expr::Literal {
                    value: LitValue::Float(f),
                    span,
                })
            }
            TokenKind::BoolLit(b) => {
                let b = b;
                let span = self.advance().span;
                Ok(Expr::Literal {
                    value: LitValue::Bool(b),
                    span,
                })
            }
            TokenKind::Ident(name) => {
                let name = name.clone();
                let start = self.advance().span;
                // Check for function call
                if *self.peek() == TokenKind::LParen {
                    self.advance(); // consume '('
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
            TokenKind::If => self.parse_if_expr(),
            ref kind if prefix_bp(kind).is_some() => {
                let bp = prefix_bp(self.peek()).unwrap();
                let op_token = self.advance().clone();
                let op = match op_token.kind {
                    TokenKind::Minus => UnaryOp::Neg,
                    TokenKind::Not => UnaryOp::Not,
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
        _ => unreachable!("not a binary operator: {:?}", kind),
    }
}

pub fn parse(tokens: Vec<Token>) -> Result<Program, Vec<CompileError>> {
    let mut parser = Parser::new(tokens);
    parser.parse_program()
}
