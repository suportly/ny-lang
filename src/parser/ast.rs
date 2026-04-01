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
    StructDef {
        name: String,
        fields: Vec<(String, TypeAnnotation)>,
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
        target: AssignTarget,
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
    ForRange {
        var: String,
        start: Expr,
        end: Expr,
        inclusive: bool,
        body: Expr,
        span: Span,
    },
    Break {
        span: Span,
    },
    Continue {
        span: Span,
    },
}

/// Target for assignment — can be a simple variable, index, field, or deref
#[derive(Debug, Clone)]
pub enum AssignTarget {
    Var(String),
    Index(Box<Expr>, Box<Expr>), // arr[i]
    Field(Box<Expr>, String),    // obj.field
    Deref(Box<Expr>),            // *ptr
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
    ArrayLit {
        elements: Vec<Expr>,
        span: Span,
    },
    Index {
        object: Box<Expr>,
        index: Box<Expr>,
        span: Span,
    },
    FieldAccess {
        object: Box<Expr>,
        field: String,
        span: Span,
    },
    StructInit {
        name: String,
        fields: Vec<(String, Expr)>,
        span: Span,
    },
    AddrOf {
        operand: Box<Expr>,
        span: Span,
    },
    Deref {
        operand: Box<Expr>,
        span: Span,
    },
    MethodCall {
        object: Box<Expr>,
        method: String,
        args: Vec<Expr>,
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
            | Expr::Block { span, .. }
            | Expr::ArrayLit { span, .. }
            | Expr::Index { span, .. }
            | Expr::FieldAccess { span, .. }
            | Expr::StructInit { span, .. }
            | Expr::AddrOf { span, .. }
            | Expr::Deref { span, .. }
            | Expr::MethodCall { span, .. } => *span,
        }
    }
}

#[derive(Debug, Clone)]
pub enum LitValue {
    Int(i128),
    Float(f64),
    Bool(bool),
    Str(String),
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
pub enum TypeAnnotation {
    Named {
        name: String,
        span: Span,
    },
    Array {
        elem: Box<TypeAnnotation>,
        size: usize,
        span: Span,
    },
    Pointer {
        inner: Box<TypeAnnotation>,
        span: Span,
    },
}

impl TypeAnnotation {
    pub fn span(&self) -> Span {
        match self {
            TypeAnnotation::Named { span, .. }
            | TypeAnnotation::Array { span, .. }
            | TypeAnnotation::Pointer { span, .. } => *span,
        }
    }

    pub fn name_str(&self) -> &str {
        match self {
            TypeAnnotation::Named { name, .. } => name.as_str(),
            TypeAnnotation::Array { .. } => "<array>",
            TypeAnnotation::Pointer { .. } => "<pointer>",
        }
    }
}
