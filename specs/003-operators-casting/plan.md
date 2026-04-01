# Implementation Plan: Operators, Casting & Comments (Phase 3)

**Branch**: `003-operators-casting` | **Date**: 2026-04-01 | **Spec**: [spec.md](specs/003-operators-casting/spec.md)
**Input**: Feature specification from `/specs/003-operators-casting/spec.md`

## Summary

Add compound assignment operators (`+=`, `-=`, etc.), bitwise operators (`& | ^ << >> ~`), type casting (`expr as T`), and block comments (`/* */`). These are pure syntax/operator extensions — no new runtime concepts, no new memory model, no new data structures. All changes are within existing compiler pipeline files.

## Technical Context

**Language/Version**: Rust 1.75+ (2021 edition)
**Primary Dependencies**: inkwell (LLVM 18), codespan-reporting, clap
**Storage**: N/A
**Testing**: cargo test (unit + integration)
**Target Platform**: x86-64 Linux
**Project Type**: CLI compiler
**Performance Goals**: No performance impact — same compilation speed
**Constraints**: No new runtime dependencies; all changes are compile-time
**Scale/Scope**: 20 functional requirements, 4 user stories, extends lexer/parser/semantic/codegen

## Constitution Check

*No constitution file. Proceeding without gates.*

## Project Structure

No new files. All changes extend existing source files:

```text
src/
├── lexer/
│   ├── token.rs         # Add: Pipe, Caret, Tilde, LtLt, GtGt, PlusAssign, MinusAssign, StarAssign, SlashAssign, PercentAssign, AmpAssign, PipeAssign, CaretAssign, LtLtAssign, GtGtAssign, As tokens
│   └── mod.rs           # Add: scan new tokens, block comment scanning, | as Pipe (not error)
├── parser/
│   ├── ast.rs           # Add: BinOp::BitAnd/BitOr/BitXor/Shl/Shr, UnaryOp::BitNot, Expr::Cast
│   ├── precedence.rs    # Add: bitwise operator binding powers
│   └── mod.rs           # Add: parse compound assignment (desugar), parse `as T`, parse `~` prefix
├── semantic/
│   ├── typechecker.rs   # Add: bitwise type rules (integers only), cast validation matrix
│   └── resolver.rs      # Add: resolve Cast expression
├── codegen/
│   └── mod.rs           # Add: LLVM bitwise ops, cast intrinsics, compile_cast()
└── common/
    └── types.rs         # No changes needed
```

## Complexity Tracking

> No violations to justify. This is a straightforward operator extension.
