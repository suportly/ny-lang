# Research: Strings, Enums & Tuples (Phase 4)

**Date**: 2026-04-01
**Feature**: 004-strings-enums-tuples

## R1: String Concatenation with Heap Allocation

**Decision**: String concatenation (`+`) calls `malloc(a.len + b.len)`, then `memcpy` both halves, returning a new `{ptr, len}` struct. Declare `extern malloc`, `memcpy`, `free` in codegen.

**Rationale**: This is the simplest correct approach. The allocated memory leaks until Phase 5 adds `defer`/`free`. This is acceptable for a language milestone — Go's early versions also had memory management gaps.

**LLVM patterns**:
- `module.add_function("malloc", ptr_ty.fn_type(&[i64_ty.into()], false), None)`
- `module.add_function("memcpy", ptr_ty.fn_type(&[ptr_ty.into(), ptr_ty.into(), i64_ty.into()], false), None)`
- Concat: `new_ptr = malloc(a.len + b.len)`, `memcpy(new_ptr, a.ptr, a.len)`, `memcpy(new_ptr + a.len, b.ptr, b.len)`, return `{new_ptr, a.len + b.len}`

## R2: String Methods (len, substr, compare)

**Decision**: Implement as built-in method dispatches in codegen, similar to print/println.

- **len()**: Extract field 1 (length) from the `{ptr, len}` struct. Zero overhead.
- **substr(start, end)**: Return `{ptr + start, end - start}`. View into original — no allocation. Bounds clamped to `min(end, len)`.
- **compare (== !=)**: Compare lengths first. If equal, call `memcmp(a.ptr, b.ptr, a.len)`. Return `result == 0` for `==`.

**LLVM patterns**:
- `memcmp` declared as extern: `i32_ty.fn_type(&[ptr_ty, ptr_ty, i64_ty], false)`
- String comparison in `compile_binop`: detect when both operands are `NyType::Str`, emit length check + memcmp.

## R3: Enum Representation

**Decision**: C-style enums as `i32` discriminants. Variant `N` has discriminant `N` (0-indexed). No data payloads.

**Rationale**: Simplest possible enum. Each variant is just an integer. `match` compiles to LLVM `build_switch`. Comparison is integer comparison.

**Type system**:
- `NyType::Enum { name: String, variants: Vec<String> }`
- Variant access: `Color::Red` → integer constant `0`, `Color::Green` → `1`, etc.
- LLVM type: `i32`

## R4: Match Expression Compilation

**Decision**: Compile match to LLVM `build_switch` for integer/enum subjects, with basic blocks for each arm.

**LLVM pattern**:
```
%subject = <evaluate match subject>
switch i32 %subject, label %default [
    i32 0, label %arm_0
    i32 1, label %arm_1
    i32 2, label %arm_2
]

arm_0:
    %val0 = <evaluate arm 0 body>
    br label %merge

arm_1:
    %val1 = <evaluate arm 1 body>
    br label %merge

merge:
    %result = phi i32 [%val0, %arm_0], [%val1, %arm_1], [%val2, %arm_2], [%default_val, %default]
```

**Exhaustiveness checking**: In the semantic pass, collect all variant names from the enum definition. For each match arm, remove the covered variant. If any remain and no `_` arm exists, report error.

## R5: Tuple Representation

**Decision**: Tuples are anonymous LLVM struct types. `(i32, bool)` → `{ i32, i1 }`. Tuple index `.0` → `build_extract_value(struct, 0)` or `build_struct_gep(0)`.

**Rationale**: Exactly how structs already work, but anonymous and indexed by position instead of name.

**Type system**:
- `NyType::Tuple(Vec<NyType>)`
- `TypeAnnotation::Tuple { elements: Vec<TypeAnnotation>, span }`
- LLVM: `context.struct_type(&[field_types], false)` (anonymous, not named)

**Destructuring**: `(a, b) := expr;` desugars during parsing to:
```
__tmp := expr;
a := __tmp.0;
b := __tmp.1;
```
This avoids new codegen — reuses existing tuple index access.

## R6: New Tokens Required

| Token | Lexeme | Purpose |
|-------|--------|---------|
| `Enum` | `enum` | Enum definitions |
| `Match` | `match` | Match expressions |
| `FatArrow` | `=>` | Match arm separator |
| `Underscore` | `_` | Wildcard pattern |
| `DoubleColon` | `::` | Already exists as ColonColon — reused for enum variant access |

**Note**: `::` already exists as `ColonColon` (used for compile-time constants). It's reused for `Color::Red` syntax. The parser disambiguates by context: if followed by an identifier after a known enum type, it's variant access.
