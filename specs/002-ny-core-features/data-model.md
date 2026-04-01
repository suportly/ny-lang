# Data Model: Ny Lang Core Features (Phase 2)

**Date**: 2026-04-01
**Feature**: 002-ny-core-features

## Type System Extensions

### NyType Enum — New Variants

```
Existing (Phase 1):
  I8, I16, I32, I64, I128, U8, U16, U32, U64, U128, F32, F64, Bool, Unit, Function

New (Phase 2):
  Array { elem: Box<NyType>, size: usize }
  Struct { name: String, fields: Vec<(String, NyType)> }
  Pointer(Box<NyType>)
  Str
```

### Array Type `[N]T`

| Property | Value |
|----------|-------|
| Syntax | `[N]T` where N is compile-time integer |
| Semantics | Value type — copied on assignment/pass |
| Storage | Stack-allocated via LLVM alloca |
| LLVM repr | `[N x T]` array type |
| Element access | GEP with bounds check |
| Nesting | `[M][N]T` = array of arrays |

### Struct Type

| Property | Value |
|----------|-------|
| Syntax | `struct Name { field: Type, ... }` |
| Semantics | Value type — copied on assignment/pass |
| Storage | Stack-allocated, fields at fixed offsets |
| LLVM repr | Named struct type `%Name = type { T1, T2, ... }` |
| Methods | Static dispatch, `self` or `self: *Name` parameter |
| Nesting | Structs can contain other structs and arrays |

### Pointer Type `*T`

| Property | Value |
|----------|-------|
| Syntax | `*T` |
| Operations | `&expr` (address-of), `*ptr` (deref read/write) |
| LLVM repr | Opaque `ptr` (LLVM 18) |
| Auto-deref | `ptr.field` auto-dereferences for struct pointers |
| Arithmetic | Not supported |

### String Type `str`

| Property | Value |
|----------|-------|
| Syntax | `str` type name, `"..."` literals |
| Semantics | Immutable, copy-by-value (pointer+length pair) |
| LLVM repr | `{ ptr, i64 }` struct (pointer to bytes + byte length) |
| Literals | Global constants, null-terminated internally |
| Operations | `print`/`println` only; no concat/slice in Phase 2 |

## New AST Nodes

### Item (top-level declarations)

| New Variant | Fields |
|-------------|--------|
| StructDef | name: String, fields: Vec\<(String, TypeAnnotation)\>, span: Span |

### Stmt (statements)

| New Variant | Fields |
|-------------|--------|
| ForRange | var: String, start: Expr, end: Expr, inclusive: bool, body: Expr, span: Span |
| Break | span: Span |
| Continue | span: Span |

### Expr (expressions)

| New Variant | Fields |
|-------------|--------|
| ArrayLit | elements: Vec\<Expr\>, span: Span |
| Index | object: Box\<Expr\>, index: Box\<Expr\>, span: Span |
| FieldAccess | object: Box\<Expr\>, field: String, span: Span |
| StructInit | name: String, fields: Vec\<(String, Expr)\>, span: Span |
| AddrOf | operand: Box\<Expr\>, span: Span |
| Deref | operand: Box\<Expr\>, span: Span |
| MethodCall | object: Box\<Expr\>, method: String, args: Vec\<Expr\>, span: Span |

### TypeAnnotation Extensions

| New Variant | Description |
|-------------|-------------|
| Array | elem: Box\<TypeAnnotation\>, size: usize, span: Span |
| Pointer | inner: Box\<TypeAnnotation\>, span: Span |

## New Token Kinds

| Token | Lexeme | Category |
|-------|--------|----------|
| Struct | `struct` | Keyword |
| For | `for` | Keyword |
| In | `in` | Keyword |
| Break | `break` | Keyword |
| Continue | `continue` | Keyword |
| DotDot | `..` | Operator |
| DotDotEq | `..=` | Operator |
| Dot | `.` | Operator |
| Ampersand | `&` | Operator |
| LBracket | `[` | Punctuation |
| RBracket | `]` | Punctuation |
| StringLit(String) | `"..."` | Literal |
| ColonAssign | `:=` | Punctuation |
| ColonTildeAssign | `:~=` | Punctuation |

## Codegen Additions

### Loop Stack

```
struct LoopFrame {
    break_bb: BasicBlock,     // target for break
    continue_bb: BasicBlock,  // target for continue
}

CodeGen.loop_stack: Vec<LoopFrame>
```

### Print Runtime

```
External functions (declared, linked from libc):
  printf(fmt: *i8, ...) -> i32     — for scalar values
  write(fd: i32, buf: *i8, len: i64) -> i64  — for strings

Struct printing: codegen generates inline printf calls for each field
```

## Relationships

```
Program 1──* Item (FunctionDef | StructDef)
StructDef 1──* (field_name, TypeAnnotation)
TypeAnnotation = Named(String) | Array(elem, size) | Pointer(inner)
NyType::Struct ←→ LLVM named struct type
NyType::Array ←→ LLVM array type
NyType::Pointer ←→ LLVM opaque ptr
NyType::Str ←→ LLVM {ptr, i64} struct
```
