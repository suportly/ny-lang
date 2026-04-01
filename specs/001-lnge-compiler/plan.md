# Implementation Plan: LNGE Compiler MVP (Phases 0-3)

**Branch**: `001-lnge-compiler` | **Date**: 2026-04-01 | **Spec**: [spec.md](specs/001-lnge-compiler/spec.md)
**Input**: Feature specification from `/specs/001-lnge-compiler/spec.md`

## Summary

Build a compiler for the LNGE language MVP: a Rust-based compiler with LLVM 18 backend that compiles scalar programs (arithmetic, functions, control flow, variables with immutability enforcement) to native x86-64 Linux executables. The compiler pipeline is: source → lexer → parser (Pratt parsing) → semantic analysis (name resolution + type checking + immutability) → LLVM IR codegen → native binary via LLVM.

## Technical Context

**Language/Version**: Rust 1.75+ (2021 edition)
**Primary Dependencies**: inkwell (LLVM 18 safe Rust bindings), codespan-reporting (error diagnostics), clap (CLI argument parsing)
**Storage**: N/A (file-based I/O only — reads .lnge source, writes native ELF binaries)
**Testing**: cargo test (unit + integration), with integration tests compiling and running .lnge programs end-to-end
**Target Platform**: x86-64 Linux (host compilation only, no cross-compilation in MVP)
**Project Type**: CLI compiler
**Performance Goals**: Compile 100-line scalar program in <2s; Fibonacci(40) executable within 2x of equivalent C
**Constraints**: LLVM 18.x must be available on build system; no standard library in MVP
**Scale/Scope**: Single-file compilation only; ~16 scalar types; ~5 control flow constructs

## Constitution Check

*No constitution file found. Proceeding without constitutional gates.*

## Project Structure

### Documentation (this feature)

```text
specs/001-lnge-compiler/
├── plan.md              # This file
├── research.md          # Phase 0 output
├── data-model.md        # Phase 1 output
├── quickstart.md        # Phase 1 output
├── contracts/           # Phase 1 output (CLI contract)
└── tasks.md             # Phase 2 output (/speckit.tasks command)
```

### Source Code (repository root)

```text
src/
├── main.rs              # CLI entry point (clap-based)
├── lib.rs               # Library root, re-exports modules
├── lexer/
│   ├── mod.rs           # Lexer engine: source → token stream
│   └── token.rs         # Token types, spans, keywords
├── parser/
│   ├── mod.rs           # Parser engine: tokens → AST (Pratt parsing)
│   ├── ast.rs           # AST node definitions
│   └── precedence.rs    # Operator precedence table
├── semantic/
│   ├── mod.rs           # Semantic analysis coordinator
│   ├── resolver.rs      # Name resolution, scope management
│   └── typechecker.rs   # Type checking, immutability enforcement
├── codegen/
│   ├── mod.rs           # LLVM IR generation via inkwell
│   └── types.rs         # LNGE→LLVM type mapping
├── diagnostics/
│   └── mod.rs           # Error reporting with codespan-reporting
└── common/
    ├── mod.rs           # Shared types
    ├── span.rs          # Source location spans
    └── types.rs         # LNGE type system representation

tests/
├── integration/
│   ├── compile_run.rs   # End-to-end: compile .lnge → run binary → check output
│   └── error_tests.rs   # Compile invalid .lnge → check error messages
├── unit/
│   ├── lexer_tests.rs   # Lexer token stream tests
│   ├── parser_tests.rs  # Parser AST construction tests
│   └── semantic_tests.rs # Type check / resolution tests
└── fixtures/
    ├── valid/           # Valid .lnge programs for integration tests
    └── invalid/         # Invalid .lnge programs for error tests

Cargo.toml               # Workspace/package manifest
```

**Structure Decision**: Single Rust crate with module-per-phase architecture. Each compiler phase (lexer, parser, semantic, codegen) is a separate module under `src/`. Integration tests live in `tests/` with fixture `.lnge` files. This is the standard Rust project layout for a compiler of this size.

## Complexity Tracking

> No constitution violations to justify.
