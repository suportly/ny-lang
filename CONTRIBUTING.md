# Contributing to Ny Lang

Thanks for your interest in contributing to Ny Lang! This document explains how to get started.

## Development Setup

```bash
# Prerequisites
# - Rust 1.75+ (rustup.rs)
# - LLVM 18 (see README.md for install)
# - gcc or clang for linking

git clone https://github.com/suportly/ny-lang.git
cd ny-lang
cargo build
cargo test   # should pass 98 tests
```

## Project Structure

```
src/
├── main.rs              # CLI entry point (ny build/run/check/test/fmt)
├── lib.rs               # Compiler pipeline
├── lsp.rs               # Language Server Protocol
├── formatter.rs         # ny fmt (comment-preserving)
├── monomorphize.rs      # Generic specialization
├── lexer/               # Source → Tokens
├── parser/              # Tokens → AST (Pratt parser, error recovery)
├── semantic/            # Name resolution + type checking
│   ├── resolver.rs      # Scope analysis, "did you mean?" suggestions
│   └── typechecker.rs   # Type checking, method dispatch
├── codegen/             # AST → LLVM IR → native binary
│   ├── expr.rs          # Expression compilation (builtins, methods)
│   ├── stmt.rs          # Statement compilation
│   ├── ops.rs           # Operators (arithmetic, SIMD)
│   ├── print.rs         # print/println for all types
│   ├── builtins.rs      # Builtin function registry
│   ├── runtime_decls.rs # C runtime function declarations
│   ├── inference.rs     # Type inference
│   ├── cast.rs          # as casts
│   └── types.rs         # NyType → LLVM type mapping
├── common/              # Shared types (NyType, Span, CompileError)
└── diagnostics/         # Error printing with codespan-reporting
runtime/                 # C runtime (linked with every binary)
├── hashmap.c            # String→int hashmap
├── arena.c              # Bump/arena allocator
├── channel.c            # Bounded blocking channels
├── threadpool.c         # Thread pool + parallel iterators
└── string.c             # String helpers (split, replace, clock)
```

## Spec-Driven Workflow

Every feature follows a spec → implement → test cycle:

1. **Spec** — Write or read the spec in `specs/` (e.g., `specs/015-new-feature/spec.md`)
2. **Implement** — Follow the spec, touching the relevant compiler phases
3. **Test** — Add both positive (`tests/fixtures/valid/`) and negative (`tests/fixtures/invalid/`) tests
4. **Document** — Update CLAUDE.md and README.md

## Adding a New Feature

### Adding a new builtin function

1. **Register** in `src/codegen/builtins.rs` — add return type + name to `BUILTIN_NAMES`
2. **Declare** runtime function in `src/codegen/runtime_decls.rs` (if C-backed)
3. **Implement** in `src/codegen/expr.rs` — add codegen in the builtin dispatch section
4. **Type-check** in `src/semantic/typechecker.rs` — add return type handling
5. **Infer** in `src/codegen/inference.rs` — add to method return type inference
6. **Test** — create `tests/fixtures/valid/feature_name.ny` + add to `tests/compile_run.rs`

### Adding a new method to Vec\<T\> or str

1. **Codegen** in `src/codegen/expr.rs` — find the Vec/str method match block
2. **Type-check** in `src/semantic/typechecker.rs`
3. **Infer** in `src/codegen/inference.rs`
4. **Test** — fixture + compile_run.rs entry
5. **LSP** — update dot-completion in `src/lsp.rs` if it's a commonly-used method

### Adding a new statement type

1. **AST** in `src/parser/ast.rs` — add new `Stmt` variant
2. **Lexer** — add keyword token if needed in `src/lexer/`
3. **Parser** — handle in `parse_block_expr()` in `src/parser/mod.rs`
4. **Resolver** — add scope handling in `src/semantic/resolver.rs`
5. **Type-checker** — add in `src/semantic/typechecker.rs`
6. **Codegen** — add in `src/codegen/stmt.rs`
7. **Formatter** — add in `src/formatter.rs`

## Running Tests

```bash
cargo test                    # All tests
cargo test test_vec_sort      # Single test
cargo clippy                  # Lint (must be clean)
cargo run --bin ny -- check examples/mandelbrot.ny  # Quick type-check
```

## Code Quality

- All code must pass `cargo clippy` with zero warnings
- All tests must pass
- Keep CLAUDE.md updated when adding features
- Prefer small, focused commits

## Pull Requests

- One feature per PR
- Include test fixtures (positive and negative)
- Update CLAUDE.md and README.md if the feature is user-facing
- Run `cargo test && cargo clippy` before submitting
