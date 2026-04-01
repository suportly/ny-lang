use crate::common::Span;

#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    // Literals
    IntLit(i128),
    FloatLit(f64),
    BoolLit(bool),

    // Identifier
    Ident(String),

    // Literals (new)
    StringLit(String),

    // Keywords
    Fn,
    If,
    Else,
    While,
    Return,
    Struct,
    For,
    In,
    Break,
    Continue,

    // Operators
    Plus,
    Minus,
    Star,
    Slash,
    Percent,
    Eq,
    Ne,
    Lt,
    Gt,
    Le,
    Ge,
    And,
    Or,
    Not,
    Assign,

    // Operators (new)
    Dot,
    DotDot,
    DotDotEq,
    Ampersand,

    // Punctuation
    LParen,
    RParen,
    LBrace,
    RBrace,
    LBracket,
    RBracket,
    Comma,
    Colon,
    ColonTilde,
    ColonColon,
    ColonAssign,
    ColonTildeAssign,
    Semi,
    Arrow,

    // Special
    Eof,
}

#[derive(Debug, Clone)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
}

impl Token {
    pub fn new(kind: TokenKind, span: Span) -> Self {
        Self { kind, span }
    }
}
