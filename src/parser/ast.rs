use crate::common::Span;

#[derive(Debug, Clone)]
pub struct Program {
    pub items: Vec<Item>,
}

/// A generic type parameter, optionally with trait bounds.
/// e.g. `T`, `T: Ord`, `T: Display + Debug`
#[derive(Debug, Clone)]
pub struct TypeParam {
    pub name: String,
    pub bounds: Vec<String>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub enum Item {
    FunctionDef {
        name: String,
        is_async: bool,
        type_params: Vec<TypeParam>,
        params: Vec<Param>,
        return_type: TypeAnnotation,
        body: Expr,
        span: Span,
    },
    StructDef {
        name: String,
        type_params: Vec<TypeParam>,
        fields: Vec<(String, TypeAnnotation)>,
        span: Span,
    },
    EnumDef {
        name: String,
        type_params: Vec<TypeParam>,
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
    GpuExternBlock {
        lib: GpuLib,
        functions: Vec<GpuFnDecl>,
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
    TypeAlias {
        name: String,
        target: TypeAnnotation,
        span: Span,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GpuLib {
    Cuda,
    OpenCL,
}

#[derive(Debug, Clone)]
pub struct GpuFnDecl {
    pub name: String,
    pub params: Vec<Param>,
    pub return_type: TypeAnnotation,
    pub is_kernel: bool,
    pub span: Span,
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
    /// while let Pattern = expr { body }
    WhileLet {
        pattern: Pattern,
        expr: Expr,
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
    /// for key, value in map { body }
    ForMap {
        key_var: String,
        val_var: String,
        map_expr: Expr,
        body: Expr,
        span: Span,
    },
    /// select { var := ch.recv() => { body }, ... }
    Select {
        arms: Vec<SelectArm>,
        span: Span,
    },
}

#[derive(Debug, Clone)]
pub struct SelectArm {
    pub var: String,
    pub channel: Expr,
    pub body: Expr,
    pub span: Span,
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
        expr: Option<Box<Expr>>,
        span: Span,
    },
    Tuple {
        elements: Vec<Expr>,
        span: Span,
    },
    Array {
        elements: Vec<Expr>,
        span: Span,
    },
    Index {
        base: Box<Expr>,
        index: Box<Expr>,
        span: Span,
    },
    FieldAccess {
        base: Box<Expr>,
        field: String,
        span: Span,
    },
    MethodCall {
        receiver: Box<Expr>,
        method: String,
        args: Vec<Expr>,
        span: Span,
    },
    Lambda {
        params: Vec<Param>,
        body: Box<Expr>,
        return_type: Option<TypeAnnotation>,
        span: Span,
    },
    Match {
        expr: Box<Expr>,
        arms: Vec<MatchArm>,
        span: Span,
    },
    StructLiteral {
        name: String,
        fields: Vec<(String, Expr)>,
        span: Span,
    },
    EnumLiteral {
        enum_name: String,
        variant_name: String,
        payload: Vec<Expr>,
        span: Span,
    },
    /// Pointer dereference: `*ptr`
    Deref {
        expr: Box<Expr>,
        span: Span,
    },
    /// Address-of operator: `&expr` or `&mut expr`
    AddrOf {
        mutability: Mutability,
        expr: Box<Expr>,
        span: Span,
    },
    /// `go func(args)`
    Go {
        call: Box<Expr>,
        span: Span,
    },
    /// `expr as Type`
    Cast {
        expr: Box<Expr>,
        target_type: TypeAnnotation,
        span: Span,
    },
    /// `sizeof(Type)`
    SizeOf {
        ty: TypeAnnotation,
        span: Span,
    },
    /// `alignof(Type)`
    AlignOf {
        ty: TypeAnnotation,
        span: Span,
    },
    /// `spawn { block }`
    Spawn {
        body: Box<Expr>,
        span: Span,
    },
    /// `chan<T>()` or `chan<T>(capacity)`
    ChanInit {
        ty: TypeAnnotation,
        capacity: Option<Box<Expr>>,
        span: Span,
    },
    /// `ch.send(val)`
    ChanSend {
        channel: Box<Expr>,
        value: Box<Expr>,
        span: Span,
    },
    /// `ch.recv()`
    ChanRecv {
        channel: Box<Expr>,
        span: Span,
    },
}

#[derive(Debug, Clone)]
pub struct MatchArm {
    pub pattern: Pattern,
    pub body: Expr,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub enum Pattern {
    Literal(LitValue, Span),
    Ident(String, Span),
    EnumVariant {
        enum_name: String,
        variant_name: String,
        bindings: Vec<String>,
        span: Span,
    },
    Wildcard(Span),
}

#[derive(Debug, Clone, PartialEq)]
pub enum LitValue {
    Int(i64),
    Float(f64),
    String(String),
    Bool(bool),
    Char(char),
    Unit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    Rem,
    Eq,
    NotEq,
    Lt,
    Gt,
    LtEq,
    GtEq,
    And,
    Or,
    BitwiseAnd,
    BitwiseOr,
    BitwiseXor,
    Shl,
    Shr,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnaryOp {
    Neg,
    Not,
}

#[derive(Debug, Clone)]
pub struct TypeAnnotation {
    pub name: String,
    pub generic_args: Vec<TypeAnnotation>,
    pub is_ptr: bool,
    pub is_mut: bool,
    pub is_slice: bool,
    pub is_array: bool,
    pub array_size: Option<Box<Expr>>,
    pub span: Span,
}
