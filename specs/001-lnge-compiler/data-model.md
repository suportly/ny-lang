# Data Model: LNGE Compiler MVP

**Date**: 2026-04-01
**Feature**: 001-lnge-compiler

## Entity Overview

```
Source Text
    ↓ (Lexer)
Token Stream [Token]
    ↓ (Parser)
AST [Program → Item → Stmt → Expr]
    ↓ (Semantic Analysis)
Typed AST (annotations on AST nodes)
    ↓ (Codegen)
LLVM Module → Object File → Executable
```

## E1: Source Span

Tracks source locations for error reporting.

| Field    | Type   | Description                          |
|----------|--------|--------------------------------------|
| file_id  | usize  | Index into the file database         |
| start    | usize  | Start byte offset (inclusive)        |
| end      | usize  | End byte offset (exclusive)          |

**Validation**: `start <= end`, both within file bounds.

## E2: Token

Lexical unit produced by the lexer.

| Field | Type      | Description                      |
|-------|-----------|----------------------------------|
| kind  | TokenKind | Discriminated token type (enum)  |
| span  | Span      | Source location                   |

**TokenKind variants**:

| Category    | Variants                                                                      |
|-------------|-------------------------------------------------------------------------------|
| Literals    | `IntLit(i128)`, `FloatLit(f64)`, `BoolLit(bool)`                            |
| Identifier  | `Ident(String)`                                                               |
| Keywords    | `Fn`, `If`, `Else`, `While`, `Return`, `True`, `False`  |
| Operators   | `Plus`, `Minus`, `Star`, `Slash`, `Percent`, `Eq`, `Ne`, `Lt`, `Gt`, `Le`, `Ge`, `And`, `Or`, `Not`, `Assign` |
| Punctuation | `LParen`, `RParen`, `LBrace`, `RBrace`, `Comma`, `Colon`, `ColonTilde`, `ColonColon`, `Semi`, `Arrow` |
| Special     | `Eof`                                                                         |

## E3: AST Nodes

### Program (root)

| Field     | Type        | Description                    |
|-----------|-------------|--------------------------------|
| items     | Vec\<Item\> | Top-level declarations         |

### Item

| Variant         | Fields                                                          |
|-----------------|-----------------------------------------------------------------|
| FunctionDef     | name: String, params: Vec\<Param\>, return_type: TypeAnnotation, body: Expr, span: Span |

### Param

| Field | Type           | Description       |
|-------|----------------|-------------------|
| name  | String         | Parameter name    |
| ty    | TypeAnnotation | Parameter type    |
| span  | Span           | Source location   |

### Stmt

| Variant      | Fields                                                                              |
|--------------|-------------------------------------------------------------------------------------|
| VarDecl      | name: String, mutability: Mutability, ty: Option\<TypeAnnotation\>, init: Expr, span |
| ConstDecl    | name: String, ty: Option\<TypeAnnotation\>, value: Expr, span                       |
| Assign       | target: String, value: Expr, span                                                   |
| ExprStmt     | expr: Expr, span                                                                    |
| Return       | value: Option\<Expr\>, span                                                         |
| While        | condition: Expr, body: Expr, span                                                   |

### Mutability (enum)

| Variant   | Syntax   | Description                   |
|-----------|----------|-------------------------------|
| Immutable | `:`      | Cannot be reassigned          |
| Mutable   | `:~`     | Can be reassigned             |

### Expr

| Variant    | Fields                                                         |
|------------|----------------------------------------------------------------|
| Literal    | value: LitValue, span                                         |
| Ident      | name: String, span                                            |
| BinOp      | op: BinOp, lhs: Box\<Expr\>, rhs: Box\<Expr\>, span          |
| UnaryOp    | op: UnaryOp, operand: Box\<Expr\>, span                       |
| Call       | callee: String, args: Vec\<Expr\>, span                       |
| If         | condition: Box\<Expr\>, then_branch: Box\<Expr\>, else_branch: Option\<Box\<Expr\>\>, span |
| Block      | stmts: Vec\<Stmt\>, tail_expr: Option\<Box\<Expr\>\>, span    |

### LitValue (enum)

| Variant | Inner Type |
|---------|-----------|
| Int     | i128      |
| Float   | f64       |
| Bool    | bool      |

### BinOp (enum)

`Add`, `Sub`, `Mul`, `Div`, `Mod`, `Eq`, `Ne`, `Lt`, `Gt`, `Le`, `Ge`, `And`, `Or`

### UnaryOp (enum)

`Neg`, `Not`

### TypeAnnotation (enum)

| Variant | Description                                                   |
|---------|---------------------------------------------------------------|
| Named   | A named type: i8, i16, i32, i64, i128, u8, u16, u32, u64, u128, f32, f64, bool |

## E4: Type System (Semantic Analysis)

### LngeType (enum)

| Variant    | Description                               |
|------------|-------------------------------------------|
| I8         | 8-bit signed integer                     |
| I16        | 16-bit signed integer                    |
| I32        | 32-bit signed integer                    |
| I64        | 64-bit signed integer                    |
| I128       | 128-bit signed integer                   |
| U8         | 8-bit unsigned integer                   |
| U16        | 16-bit unsigned integer                  |
| U32        | 32-bit unsigned integer                  |
| U64        | 64-bit unsigned integer                  |
| U128       | 128-bit unsigned integer                 |
| F32        | 32-bit floating point                    |
| F64        | 64-bit floating point                    |
| Bool       | Boolean                                  |
| Unit       | Unit type (no value / void)              |
| Function   | params: Vec\<LngeType\>, ret: Box\<LngeType\> |

### Symbol

| Field      | Type        | Description                          |
|------------|-------------|--------------------------------------|
| name       | String      | Symbol name                          |
| ty         | LngeType    | Resolved type                        |
| mutability | Mutability  | Immutable or Mutable                 |
| scope      | ScopeId     | Which scope this symbol belongs to   |

### Scope

| Field    | Type                         | Description                    |
|----------|------------------------------|--------------------------------|
| id       | ScopeId                      | Unique scope identifier        |
| parent   | Option\<ScopeId\>            | Enclosing scope                |
| symbols  | HashMap\<String, Symbol\>    | Symbols declared in this scope |

**State transitions**: Scopes are pushed when entering function bodies, blocks, and loop bodies. Popped when exiting. Name resolution walks the scope chain from inner to outer.

## E5: Compiler Pipeline State

### CompileResult

| Field      | Type             | Description                        |
|------------|------------------|------------------------------------|
| success    | bool             | Whether compilation succeeded      |
| output     | Option\<PathBuf\>| Path to output executable          |
| errors     | Vec\<CompileError\> | Collected compiler errors       |

### CompileError

| Field   | Type   | Description                        |
|---------|--------|------------------------------------|
| message | String | Human-readable error description   |
| span    | Span   | Source location of the error       |
| kind    | ErrorKind | Category (Syntax, Type, Name, etc.) |

## Relationships

```
Program 1──* Item (FunctionDef)
FunctionDef 1──* Param
FunctionDef 1──1 Expr (body, always a Block)
Block 1──* Stmt
Stmt *──* Expr (various relationships)
Expr *──* Expr (recursive via Box)
Symbol *──1 Scope
Scope *──? Scope (parent chain)
```
