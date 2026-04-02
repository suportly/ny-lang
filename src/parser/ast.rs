use crate::common::Span;

#[derive(Debug, Clone)]
pub struct Program {
    pub items: Vec<Item>,
}

#[derive(Debug, Clone)]
pub enum Item {
    FunctionDef {
        name: String,
        type_params: Vec<String>,
        params: Vec<Param>,
        return_type: TypeAnnotation,
        body: Expr,
        span: Span,
    },
    StructDef {
        name: String,
        type_params: Vec<String>,
        fields: Vec<(String, TypeAnnotation)>,
        span: Span,
    },
    EnumDef {
        name: String,
        variants: Vec<EnumVariantDef>,
        span: Span,
    },
    Use {
        path: String,
        span: Span,
    },
    ExternBlock {
        functions: Vec<ExternFnDecl>,
        span: Span,
    },
    ImplBlock {
        type_name: String,
        trait_name: Option<String>,
        methods: Vec<Item>,
        span: Span,
    },
    TraitDef {
        name: String,
        methods: Vec<TraitMethodSig>,
        span: Span,
    },
}

#[derive(Debug, Clone)]
pub struct ExternFnDecl {
    pub name: String,
    pub params: Vec<Param>,
    pub return_type: TypeAnnotation,
    pub variadic: bool,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct TraitMethodSig {
    pub name: String,
    pub params: Vec<Param>,
    pub return_type: TypeAnnotation,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct EnumVariantDef {
    pub name: String,
    pub payload: Vec<TypeAnnotation>,
    pub span: Span,
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
    /// for item in collection { body } — iterates over array/slice elements
    ForIn {
        var: String,
        collection: Expr,
        body: Expr,
        span: Span,
    },
    Break {
        span: Span,
    },
    Continue {
        span: Span,
    },
    TupleDestructure {
        names: Vec<String>,
        mutability: Mutability,
        init: Expr,
        span: Span,
    },
    Defer {
        body: Expr,
        span: Span,
    },
    /// if let Pattern = expr { then } else { else }
    IfLet {
        pattern: Pattern,
        expr: Expr,
        then_body: Expr,
        else_body: Option<Expr>,
        span: Span,
    },
    Loop {
        body: Expr,
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
    Cast {
        expr: Box<Expr>,
        target_type: TypeAnnotation,
        span: Span,
    },
    Match {
        subject: Box<Expr>,
        arms: Vec<MatchArm>,
        span: Span,
    },
    TupleLit {
        elements: Vec<Expr>,
        span: Span,
    },
    TupleIndex {
        object: Box<Expr>,
        index: usize,
        span: Span,
    },
    EnumVariant {
        enum_name: String,
        variant: String,
        args: Vec<Expr>,
        span: Span,
    },
    /// expr? — try operator, extracts Ok or returns Err
    Try {
        operand: Box<Expr>,
        span: Span,
    },
    /// |params| -> RetType { body } (non-capturing lambda)
    Lambda {
        params: Vec<Param>,
        return_type: TypeAnnotation,
        body: Box<Expr>,
        span: Span,
    },
    /// arr[start..end] → creates a slice
    RangeIndex {
        object: Box<Expr>,
        start: Box<Expr>,
        end: Box<Expr>,
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
            | Expr::MethodCall { span, .. }
            | Expr::Cast { span, .. }
            | Expr::Match { span, .. }
            | Expr::TupleLit { span, .. }
            | Expr::TupleIndex { span, .. }
            | Expr::EnumVariant { span, .. }
            | Expr::RangeIndex { span, .. }
            | Expr::Lambda { span, .. }
            | Expr::Try { span, .. } => *span,
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
    BitAnd,
    BitOr,
    BitXor,
    Shl,
    Shr,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnaryOp {
    Neg,
    Not,
    BitNot,
}

#[derive(Debug, Clone)]
pub struct MatchArm {
    pub pattern: Pattern,
    pub body: Expr,
}

#[derive(Debug, Clone)]
pub enum Pattern {
    EnumVariant {
        enum_name: String,
        variant: String,
        bindings: Vec<String>,
        span: Span,
    },
    IntLit(i128, Span),
    Wildcard(Span),
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
    Tuple {
        elements: Vec<Box<TypeAnnotation>>,
        span: Span,
    },
    Slice {
        elem: Box<TypeAnnotation>,
        span: Span,
    },
    Function {
        params: Vec<Box<TypeAnnotation>>,
        ret: Box<TypeAnnotation>,
        span: Span,
    },
}

impl TypeAnnotation {
    pub fn span(&self) -> Span {
        match self {
            TypeAnnotation::Named { span, .. }
            | TypeAnnotation::Array { span, .. }
            | TypeAnnotation::Pointer { span, .. }
            | TypeAnnotation::Tuple { span, .. }
            | TypeAnnotation::Slice { span, .. }
            | TypeAnnotation::Function { span, .. } => *span,
        }
    }

    pub fn name_str(&self) -> &str {
        match self {
            TypeAnnotation::Named { name, .. } => name.as_str(),
            TypeAnnotation::Array { .. } => "<array>",
            TypeAnnotation::Pointer { .. } => "<pointer>",
            TypeAnnotation::Tuple { .. } => "<tuple>",
            TypeAnnotation::Slice { .. } => "<slice>",
            TypeAnnotation::Function { .. } => "<function>",
        }
    }
}
