use crate::common::Span;

#[derive(Debug, Clone)]
pub struct Program {
    pub items: Vec<Item>,
}

#[derive(Debug, Clone)]
pub enum Item {
    FunctionDef {
        name: String,
        params: Vec<Param>,
        return_type: TypeAnnotation,
        body: Expr,
        span: Span,
    },
}

#[derive(Debug, Clone)]
pub struct Param {
    pub name: String,
    pub ty: TypeAnnotation,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub enum Stmt {
    VarDecl {
        name: String,
        mutability: Mutability,
        ty: Option<TypeAnnotation>,
        init: Expr,
        span: Span,
    },
    ConstDecl {
        name: String,
        ty: Option<TypeAnnotation>,
        value: Expr,
        span: Span,
    },
    Assign {
        target: String,
        value: Expr,
        span: Span,
    },
    ExprStmt {
        expr: Expr,
        span: Span,
    },
    Return {
        value: Option<Expr>,
        span: Span,
    },
    While {
        condition: Expr,
        body: Expr,
        span: Span,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mutability {
    Immutable,
    Mutable,
}

#[derive(Debug, Clone)]
pub enum Expr {
    Literal {
        value: LitValue,
        span: Span,
    },
    Ident {
        name: String,
        span: Span,
    },
    BinOp {
        op: BinOp,
        lhs: Box<Expr>,
        rhs: Box<Expr>,
        span: Span,
    },
    UnaryOp {
        op: UnaryOp,
        operand: Box<Expr>,
        span: Span,
    },
    Call {
        callee: String,
        args: Vec<Expr>,
        span: Span,
    },
    If {
        condition: Box<Expr>,
        then_branch: Box<Expr>,
        else_branch: Option<Box<Expr>>,
        span: Span,
    },
    Block {
        stmts: Vec<Stmt>,
        tail_expr: Option<Box<Expr>>,
        span: Span,
    },
}

impl Expr {
    pub fn span(&self) -> Span {
        match self {
            Expr::Literal { span, .. }
            | Expr::Ident { span, .. }
            | Expr::BinOp { span, .. }
            | Expr::UnaryOp { span, .. }
            | Expr::Call { span, .. }
            | Expr::If { span, .. }
            | Expr::Block { span, .. } => *span,
        }
    }
}

#[derive(Debug, Clone)]
pub enum LitValue {
    Int(i128),
    Float(f64),
    Bool(bool),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Eq,
    Ne,
    Lt,
    Gt,
    Le,
    Ge,
    And,
    Or,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnaryOp {
    Neg,
    Not,
}

#[derive(Debug, Clone)]
pub struct TypeAnnotation {
    pub name: String,
    pub span: Span,
}
