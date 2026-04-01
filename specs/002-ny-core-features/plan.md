# Implementation Plan: Ny Lang Core Features (Phase 2)

**Branch**: `002-ny-core-features` | **Date**: 2026-04-01 | **Spec**: [spec.md](specs/002-ny-core-features/spec.md)
**Input**: Feature specification from `/specs/002-ny-core-features/spec.md`

## Summary

Extend the Ny compiler with arrays `[N]T`, structs with methods, pointers `*T`, strings `str` with `print`/`println`, `for` range loops, `break`/`continue`, and type inference `:=`/`:~=`. All features are stack-allocated, statically dispatched, and compile to efficient LLVM IR. This transforms Ny from a scalar-only language into one capable of real computation (dot products, matrix multiplication, data processing).

## Technical Context

**Language/Version**: Rust 1.75+ (2021 edition)
**Primary Dependencies**: inkwell (LLVM 18 via llvm18-1-force-dynamic), codespan-reporting, clap
**Storage**: N/A (file-based I/O only)
**Testing**: cargo test (unit + integration), end-to-end compile-and-run tests
**Target Platform**: x86-64 Linux
**Project Type**: CLI compiler
**Performance Goals**: Array-heavy programs within 1.5x of C -O2; compile 200-line program in <3s
**Constraints**: Stack-allocated only (no heap); no generics; no closures; single-file programs
**Scale/Scope**: 28 new functional requirements across 6 feature areas; extends all 6 compiler phases

## Constitution Check

*No constitution file found. Proceeding without constitutional gates.*

## Project Structure

### Documentation (this feature)

```text
specs/002-ny-core-features/
├── plan.md              # This file
├── research.md          # Phase 0 output
├── data-model.md        # Phase 1 output
├── quickstart.md        # Phase 1 output
├── contracts/           # Phase 1 output (extended CLI + syntax contracts)
└── tasks.md             # Phase 2 output (/speckit.tasks command)
```

### Source Code (repository root)

Changes extend the existing Phase 1 structure — no new directories needed:

```text
src/
├── main.rs              # CLI entry point — no changes needed
├── lib.rs               # Library root — no changes needed
├── lexer/
│   ├── mod.rs           # Add: For, In, Break, Continue, Struct keywords; DotDot, DotDotEq, Dot, Ampersand tokens; string literal scanning
│   └── token.rs         # Add: new TokenKind variants
├── parser/
│   ├── mod.rs           # Add: parse_struct_def, parse_for_range, parse_array_literal, parse_index_expr, parse_field_access, parse_address_of, parse_deref, parse_break, parse_continue, inferred decl
│   ├── ast.rs           # Add: Item::StructDef, Stmt::ForRange/Break/Continue, Expr::ArrayLit/Index/FieldAccess/AddrOf/Deref/StructInit/MethodCall, TypeAnnotation::Array/Pointer/Named(struct)
│   └── precedence.rs    # No changes — . and [] handled as postfix in parse_prefix/parse_expr
├── semantic/
│   ├── mod.rs           # No structural changes
│   ├── resolver.rs      # Add: struct type registration, field resolution, method resolution, pointer type handling, loop depth tracking for break/continue
│   └── typechecker.rs   # Add: array type checking, struct field/method type checking, pointer deref type checking, for-range validation, inference rules
├── codegen/
│   ├── mod.rs           # Add: array alloca+GEP+bounds check, struct alloca+struct_gep, pointer load/store, for-range with loop stack, break/continue, print/println via printf/write, string globals
│   └── types.rs         # Add: NyType::Array/Struct/Pointer/Str → LLVM type mapping
├── diagnostics/
│   └── mod.rs           # No changes needed — already handles all error types
└── common/
    ├── mod.rs           # No changes needed
    ├── span.rs          # No changes needed
    └── types.rs         # Add: NyType::Array { elem, size }, NyType::Struct { name, fields }, NyType::Pointer(Box<NyType>), NyType::Str

tests/
├── compile_run.rs       # Extend with array, struct, pointer, print, for-loop tests
├── error_tests.rs       # Extend with new error case tests
└── fixtures/
    ├── valid/           # Add: arrays.ny, structs.ny, pointers.ny, hello_print.ny, for_range.ny, inference.ny, matrix.ny
    └── invalid/         # Add: array_bounds.ny, struct_recursive.ny, break_outside_loop.ny, type_mismatch_array.ny
```

**Structure Decision**: Same single Rust crate, same module layout. All changes are extensions to existing files + new test fixtures. No new source modules needed.

## Complexity Tracking

> No constitution violations to justify.
