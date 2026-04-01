# lnge Development Guidelines

Last updated: 2026-04-01

## Active Technologies
- Rust 1.75+ (2021 edition) + inkwell (LLVM 18 via llvm18-1-force-dynamic), codespan-reporting, clap

## Project Structure

```text
src/
├── main.rs              # CLI (ny build, ny test)
├── lib.rs               # Compiler pipeline
├── lexer/               # Tokenization
├── parser/              # Pratt parser → AST
├── semantic/            # Name resolution + type checking
├── codegen/             # LLVM IR generation
├── common/              # Types, spans, errors
└── diagnostics/         # Error reporting
specs/                   # Feature specifications (001–014)
tests/
├── compile_run.rs       # Integration tests (28)
├── error_tests.rs       # Negative tests (10)
└── fixtures/            # .ny test programs
```

## Commands

```bash
cargo test               # Run all tests
cargo clippy             # Lint
cargo build --release    # Release binary
ny build file.ny         # Compile .ny to executable
ny test file.ny          # Run test_* functions
```

## Code Style

Rust 1.75+ (2021 edition): Follow standard conventions

## Implemented Phases

| Phase | Feature | Spec |
|-------|---------|------|
| 1 | Compiler MVP: scalars, functions, control flow | specs/001-lnge-compiler/ |
| 2 | Arrays, structs, pointers, strings, for loops | specs/002-ny-core-features/ |
| 3 | Compound assignment, bitwise, casting, block comments | specs/003-operators-casting/ |
| 4 | String ops, enums, match, tuples | specs/004-strings-enums-tuples/ |
| 5 | Heap memory: alloc/free/sizeof, defer | specs/005-heap-memory/ |
| 6 | Tagged unions (data-carrying enums), loop keyword | specs/006-tagged-unions-loop/ |
| 7 | Impl blocks, pub keyword | specs/007-impl-blocks/ |
| 8 | Trait definitions, impl Trait for Type | specs/008-traits/ |
| 9 | Slice type []T | specs/009-slices/ |
| 10 | File I/O: fopen/fclose/fwrite_str/fread_byte, exit | specs/010-file-io/ |
| 11 | Unsafe pointer operations | specs/011-unsafe-pointers/ |
| 12 | Concurrency foundation: sleep_ms | specs/012-concurrency-foundation/ |
| 13 | SIMD infrastructure (prepared) | specs/013-simd-infrastructure/ |
| 14 | Test framework: ny test | specs/014-test-framework/ |

## Builtin Functions

| Function | Signature | Phase |
|----------|-----------|-------|
| print | (any...) -> () | 2 |
| println | (any...) -> () | 2 |
| alloc | (i32) -> *u8 | 5 |
| free | (*u8) -> () | 5 |
| sizeof | (any) -> i64 | 5 |
| fopen | (str, str) -> *u8 | 10 |
| fclose | (*u8) -> i32 | 10 |
| fwrite_str | (*u8, str) -> i32 | 10 |
| fread_byte | (*u8) -> i32 | 10 |
| exit | (i32) -> ! | 10 |
| sleep_ms | (i32) -> () | 12 |

## Reserved Keywords

fn, if, else, while, for, in, return, struct, break, continue, as, enum, match,
defer, pub, use, mod, trait, impl, loop, unsafe

## Roadmap

See [specs/ROADMAP.md](specs/ROADMAP.md) for the full strategic roadmap.
