# lnge Development Guidelines

Last updated: 2026-04-01

## Active Technologies
- Rust 1.75+ (2021 edition) + inkwell (LLVM 18 via llvm18-1-force-dynamic), codespan-reporting, clap

## Project Structure

```text
src/
├── main.rs              # CLI (ny build, ny test)
├── lib.rs               # Compiler pipeline + module resolution
├── monomorphize.rs      # Generic function specialization
├── lexer/               # Tokenization
├── parser/              # Pratt parser → AST
├── semantic/            # Name resolution + type checking
├── codegen/             # LLVM IR generation
├── common/              # Types, spans, errors
└── diagnostics/         # Error reporting
runtime/
└── hashmap.c            # C runtime (HashMap implementation)
specs/                   # Feature specifications (001–014)
tests/
├── compile_run.rs       # Integration tests (47)
├── error_tests.rs       # Negative tests (11)
└── fixtures/            # .ny test programs
```

## Commands

```bash
cargo test               # Run all tests (58 total)
cargo clippy             # Lint
cargo build --release    # Release binary
ny build file.ny         # Compile .ny to executable
ny build file.ny -O 2    # Compile with optimization
ny test file.ny          # Run test_* functions
ny fmt file.ny           # Print formatted source
ny fmt file.ny --write   # Format in-place
ny fmt file.ny --check   # Check if formatted (exit 1 if not)
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
| 9 | Slice type []T + range indexing | specs/009-slices/ |
| 10 | File I/O: fopen/fclose/fwrite_str/fread_byte, exit | specs/010-file-io/ |
| 11 | Unsafe pointers, pointer arithmetic | specs/011-unsafe-pointers/ |
| 12 | Concurrency foundation: sleep_ms | specs/012-concurrency-foundation/ |
| 13 | SIMD infrastructure (prepared) | specs/013-simd-infrastructure/ |
| 14 | Test framework: ny test | specs/014-test-framework/ |

## Additional Features (beyond original phases)

- Generic functions with monomorphization: `fn max<T>(a: T, b: T) -> T`
- Module system: `use "module.ny";`
- Extern C FFI: `extern { fn abs(x: i32) -> i32; }`
- Vec<T> dynamic arrays: push/pop/get/set/len/sort/reverse/clear with auto-grow
- HashMap (str->i32): C runtime backed
- Capturing closures: `|x: i32| -> i32 { x * n }`
- for-in iteration: `for item in collection { ... }`
- ? operator: `val := divide(10, 0)?;`
- if let: `if let Option::Some(v) = expr { ... }`
- Void functions: `fn greet() { ... }` without -> ()
- Pointer arithmetic: `ptr + offset`, `*(ptr + n)`
- String methods: `.len()`, `.substr()`, `.char_at()`, `.contains()`, `.starts_with()`, `.ends_with()`, `.index_of()`

## Builtin Functions

| Function | Signature | Description |
|----------|-----------|-------------|
| print | (any...) -> () | Print to stdout |
| println | (any...) -> () | Print with newline |
| alloc | (i32) -> *u8 | Heap allocation |
| free | (*u8) -> () | Free heap memory |
| sizeof | (any) -> i64 | Size in bytes |
| vec_new | () -> Vec<T> | Create empty vector |
| map_new | () -> *u8 | Create empty hashmap |
| map_insert | (*u8, str, i32) -> () | Insert key-value |
| map_get | (*u8, str) -> i32 | Get by key |
| map_contains | (*u8, str) -> bool | Check key exists |
| map_len | (*u8) -> i64 | Map size |
| fopen | (str, str) -> *u8 | Open file |
| fclose | (*u8) -> i32 | Close file |
| fwrite_str | (*u8, str) -> i32 | Write string to file |
| fread_byte | (*u8) -> i32 | Read byte |
| read_line | () -> str | Read stdin line |
| int_to_str | (i32) -> str | Int to string |
| str_to_int | (str) -> i32 | String to int |
| exit | (i32) -> ! | Exit process |
| sleep_ms | (i32) -> () | Sleep milliseconds |

## Reserved Keywords

fn, if, else, while, for, in, return, struct, break, continue, as, enum, match,
defer, pub, use, mod, trait, impl, loop, unsafe, extern, let

## Roadmap

See [specs/ROADMAP.md](specs/ROADMAP.md) for the full strategic roadmap.
