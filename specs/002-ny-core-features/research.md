# Research: Ny Lang Core Features (Phase 2)

**Date**: 2026-04-01
**Feature**: 002-ny-core-features

## R1: Array Implementation via LLVM

**Decision**: Use LLVM array types (`[N x T]`) with alloca for stack allocation, GEP for element access, and explicit bounds checking before each index operation.

**Rationale**: LLVM array types map directly to Ny's `[N]T` semantics. Stack allocation via alloca is zero-cost. GEP (getelementptr) with two indices `[0, i]` gives a pointer to element `i`. Bounds checking adds a single integer comparison + conditional branch before each access — negligible overhead, and LLVM's optimization passes can eliminate redundant checks in loops.

**Key patterns**:
- `elem_ty.array_type(n)` creates the LLVM type
- `builder.build_alloca(arr_ty, name)` stack-allocates
- `unsafe { builder.build_in_bounds_gep(arr_ty, ptr, &[zero, idx], name) }` gets element pointer
- Bounds check: `build_int_compare(ULT, idx, len)` → branch to panic or continue
- Value passing: `build_load(arr_ty, ptr)` loads entire array as `ArrayValue`

**Alternatives considered**:
- Heap allocation: Rejected — spec mandates stack-only in this phase
- No bounds checking: Rejected — spec mandates runtime checks (FR-004)

## R2: Struct Implementation via LLVM

**Decision**: Use named LLVM struct types via `context.opaque_struct_type(name)` + `set_body()`. Fields accessed via `build_struct_gep`. Methods are regular functions with an explicit `self` parameter, resolved by the compiler to dot-call syntax.

**Rationale**: Named struct types allow forward references and produce readable LLVM IR. `build_struct_gep` validates field indices at Rust compile time. Methods as static functions with `self` parameter avoid vtable overhead — consistent with the "no dynamic dispatch" constraint.

**Key patterns**:
- `context.opaque_struct_type("Vec2")` → `named.set_body(&[f64, f64], false)`
- `builder.build_struct_gep(struct_ty, ptr, field_idx, name)` — safe, checked
- Method call `v.length()` desugars to `Vec2_length(v)` in codegen
- Pointer receiver `self: *Vec2` enables mutation without copying

## R3: Pointer Implementation

**Decision**: Use LLVM opaque pointers (`context.ptr_type(AddressSpace::default())`). Address-of returns the alloca pointer directly. Dereference uses `build_load`/`build_store` with the known pointee type.

**Rationale**: LLVM 18 uses opaque pointers exclusively — all pointers are `ptr` with no encoded element type. The compiler must track pointee types in its own type system (`NyType::Pointer(Box<NyType>)`). This is simpler than typed pointers and matches LLVM 18's architecture.

**Key patterns**:
- `&x` → return `x`'s alloca pointer (already a `PointerValue`)
- `*p` read → `builder.build_load(pointee_ty, p, name)`
- `*p = v` write → `builder.build_store(p, v)`
- Auto-deref for `ptr.field` → `build_struct_gep(struct_ty, ptr, idx, name)`
- No pointer arithmetic — only address-of, deref, and passing

## R4: String and Print Implementation

**Decision**: Use `printf` from libc for `print`/`println` on scalars. Use `write(1, ptr, len)` for string output. String literals stored as global constants with `str` represented as `{ptr, len}` struct.

**Rationale**: `printf` handles all numeric formatting (`%d`, `%ld`, `%f`) with zero implementation effort. For strings, `write` syscall is more appropriate (no format string parsing). String constants via `builder.build_global_string_ptr` or `module.add_global`. The `str` type as `{ptr, len}` is standard (Rust slices, Go strings).

**Key patterns**:
- `module.add_function("printf", i32_ty.fn_type(&[ptr_ty], true), None)` — variadic
- `builder.build_global_string_ptr("Hello\n", "lit")` for constant strings
- `println(42)` → `printf("%d\n", 42)`
- `println("hello")` → `write(1, str_ptr, str_len)` + `write(1, "\n", 1)`
- `println(true)` → branch on bool, print "true\n" or "false\n"
- `println(v)` for structs → print each field with format `StructName { f1: v1, f2: v2 }`

**Alternatives considered**:
- Custom print runtime: Rejected — too much work for Phase 2; printf is universally available
- `puts`: Only handles strings, no formatting for numbers

## R5: For-Range Loop Implementation

**Decision**: Four basic blocks: `for_cond`, `for_body`, `for_inc`, `for_exit`. Loop variable as alloca (promoted to phi by `mem2reg`). `break` branches to `for_exit`, `continue` branches to `for_inc`. Loop stack (`Vec<LoopFrame>`) tracks targets for nested loops.

**Rationale**: Alloca-based loop variable is simpler than manual phi nodes and produces identical optimized code after `mem2reg`. The four-block structure cleanly separates condition check, body, increment, and exit — making `break` and `continue` trivial unconditional branches.

**Key patterns**:
- Exclusive range `0..n`: condition uses `IntPredicate::SLT` (signed less than)
- Inclusive range `0..=n`: condition uses `IntPredicate::SLE` (signed less or equal)
- `break` → `build_unconditional_branch(loop_stack.last().break_bb)`
- `continue` → `build_unconditional_branch(loop_stack.last().continue_bb)`
- While loops updated to push `LoopFrame { break_bb: exit, continue_bb: cond }`

## R6: Type Inference

**Decision**: `:=` and `:~=` are syntactic sugar — the parser produces `VarDecl` nodes with `ty: None`. The type checker infers the type from the initialization expression using existing inference logic (already partially implemented in Phase 1).

**Rationale**: The Phase 1 type checker already falls back to `init_ty` when no type annotation is present. `:=` just needs parser support for the new syntax; the inference logic is already there. Default types: `i32` for integer literals, `f64` for float literals, `bool` for booleans — matching the existing behavior.

**Key patterns**:
- `:=` → `VarDecl { mutability: Immutable, ty: None, init: expr }`
- `:~=` → `VarDecl { mutability: Mutable, ty: None, init: expr }`
- Type checker infers type from `init` expression, same as current behavior when `ty` is `None`

## R7: New Tokens Required

**Decision**: Add the following tokens to the lexer:

| Token | Lexeme | Purpose |
|-------|--------|---------|
| `Struct` | `struct` | Struct definitions |
| `For` | `for` | For loops |
| `In` | `in` | For-in syntax |
| `Break` | `break` | Loop break |
| `Continue` | `continue` | Loop continue |
| `DotDot` | `..` | Exclusive range |
| `DotDotEq` | `..=` | Inclusive range |
| `Dot` | `.` | Field access |
| `Ampersand` | `&` | Address-of |
| `LBracket` | `[` | Array type/literal/index |
| `RBracket` | `]` | Array close |
| `StringLit(String)` | `"..."` | String literals |
| `ColonAssign` | `:=` | Inferred immutable decl |
| `ColonTildeAssign` | `:~=` | Inferred mutable decl |
