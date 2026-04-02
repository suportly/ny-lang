# Implementation Plan: Strings, Enums & Tuples (Phase 4)

**Branch**: `004-strings-enums-tuples` | **Date**: 2026-04-01 | **Spec**: [spec.md](specs/004-strings-enums-tuples/spec.md)

## Summary

Add string operations (len, concat, compare, substr), C-style enums with match expressions, and tuple types with multi-return and destructuring. String concatenation introduces the first heap allocation (malloc). Enums are integer discriminants with exhaustive match. Tuples are anonymous LLVM struct types.

## Technical Context

**Language/Version**: Rust 1.75+ (2021 edition)
**Primary Dependencies**: inkwell (LLVM 18), codespan-reporting, clap
**Testing**: cargo test (integration tests)
**Target Platform**: x86-64 Linux
**Project Type**: CLI compiler
**New runtime dependency**: libc malloc/free (for string concatenation)
**Scale/Scope**: 23 functional requirements across 4 feature areas

## Constitution Check

*No constitution file. Proceeding without gates.*

## Project Structure

All changes extend existing files. No new source modules needed:

```text
src/
├── lexer/
│   ├── token.rs         # Add: Enum, Match, FatArrow (=>), Underscore (_), DoubleColon (::) tokens
│   └── mod.rs           # Add: scan =>, ::, new keywords (enum, match)
├── parser/
│   ├── ast.rs           # Add: Item::EnumDef, Expr::Match/TupleLit/TupleIndex/EnumVariant, Stmt::TupleDestructure, TypeAnnotation::Tuple
│   ├── precedence.rs    # No changes
│   └── mod.rs           # Add: parse enum, match, tuple literal/type/destructure, string method calls dispatch
├── semantic/
│   ├── resolver.rs      # Add: enum registration, match exhaustiveness, tuple type handling
│   └── typechecker.rs   # Add: string op type rules, enum/match type checking, tuple type checking
├── codegen/
│   ├── mod.rs           # Add: enum as i32 discriminant, match as switch, tuple as anon struct, string concat (malloc+memcpy), str.len/substr/compare
│   └── types.rs         # Add: NyType::Enum, NyType::Tuple → LLVM types
└── common/
    └── types.rs         # Add: NyType::Enum { name, variants }, NyType::Tuple(Vec<NyType>)
```

## Complexity Tracking

> No violations to justify.
