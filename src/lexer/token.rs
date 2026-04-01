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
    As,
    Enum,
    Match,
    Defer,
    Pub,
    Use,
    Mod,
    Trait,
    Impl,
    Loop,
    Unsafe,
    Extern,

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

    // Operators (Phase 2)
    Dot,
    DotDot,
    DotDotEq,
    Ampersand,

    // Operators (Phase 3)
    Pipe,
    Caret,
    Tilde,
    LtLt,
    GtGt,
    PlusAssign,
    MinusAssign,
    StarAssign,
    SlashAssign,
    PercentAssign,
    AmpAssign,
    PipeAssign,
    CaretAssign,
    LtLtAssign,
    GtGtAssign,

    // Operators (Phase 4)
    FatArrow,

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

    // Phase 4
    Underscore,
    // Phase C: Try operator
    Question,

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
