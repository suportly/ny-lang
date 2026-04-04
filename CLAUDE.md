# lnge Development Guidelines

Last updated: 2026-04-04

## Active Technologies
- Rust 1.75+ (2021 edition) + inkwell (LLVM 18 via llvm18-1-force-dynamic), codespan-reporting, clap, serde/serde_json

## Project Structure

```text
src/
├── main.rs              # CLI (ny build/run/check/test/fmt/repl + pkg)
├── lib.rs               # Compiler pipeline + module resolution
├── lsp.rs               # Language Server Protocol (6 capabilities)
├── formatter.rs         # ny fmt (comment-preserving)
├── monomorphize.rs      # Generic function specialization
├── lexer/               # Tokenization
├── parser/              # Pratt parser → AST (error recovery)
├── semantic/            # Name resolution + type checking
├── codegen/             # LLVM IR generation
├── pkg/                 # Package manager (ny pkg)
├── common/              # Types, spans, errors
└── diagnostics/         # Error reporting (codespan-reporting)
runtime/
├── hashmap.c            # str→i32 HashMap
├── hashmap_generic.c    # Generic HashMap<K,V>
├── arena.c              # Arena allocator
├── channel.c            # Bounded blocking channels
├── threadpool.c         # Thread pool + parallel iterators
├── string.c             # String helpers + clock + stack traces
├── json.c               # JSON parser
├── tensor.c             # Tensor<f64> matrix operations
├── future.c             # Async/await runtime (NyFuture)
├── gc.c                 # Mark-and-sweep garbage collector
├── chan.c               # Generic typed channels (chan<T>)
└── error.c              # Error handling with string messages
editors/
└── vscode/              # VS Code extension + LSP client
specs/                   # Feature specifications
tests/
├── compile_run.rs       # Integration tests (126)
├── error_tests.rs       # Negative tests (16)
└── fixtures/            # .ny test programs
benchmarks/              # 7 benchmarks with C + Go equivalents
```

## Commands

```bash
cargo test               # Run all tests (142 total)
cargo clippy             # Lint
cargo build --release    # Release binary
ny build file.ny         # Compile .ny to executable
ny build file.ny -O 2    # Compile with optimization (release mode: no bounds checks)
ny build file.ny --target wasm32  # Compile to WebAssembly
ny run file.ny           # Compile and run in one step
ny check file.ny         # Type-check without compiling (shows timing stats)
ny test file.ny          # Run test_* functions
ny fmt file.ny           # Print formatted source
ny fmt file.ny --write   # Format in-place
ny fmt file.ny --check   # Check if formatted (exit 1 if not)
ny repl                  # Interactive REPL
ny pkg init              # Create ny.pkg manifest
ny pkg add <url>         # Add git dependency
ny pkg build             # Fetch all dependencies
ny pkg remove <name>     # Remove dependency
ny pkg list              # List dependencies
```

## Implemented Features

### Language Core
- 14 scalar types (i8-i128, u8-u128, f32, f64, bool, str)
- Arrays [N]T, Slices []T, Tuples (T, U)
- Structs + impl blocks + methods
- Enums (tagged unions) + pattern matching (match, if let, while let)
- Generics with monomorphization: `fn max<T>(a: T, b: T) -> T`
- Trait definitions + impl Trait for Type + trait bounds enforcement
- **`dyn Trait`**: dynamic dispatch via vtables + fat pointers (`{data_ptr, vtable_ptr}`)
- Operator overloading: `impl Vec2 { fn add(self, other) -> Vec2 }` → `a + b`
- Closures (capturing): `|x: i32| -> i32 { x * n }`
- Async/await: `async fn compute() -> i32 { ... }` + `await future`
- ? operator: `val := divide(10, 0)?;`
- Module system: `use "module.ny";`
- Extern C FFI: `extern { fn abs(x: i32) -> i32; }`
- f-string interpolation: `f"value is {expr}"`

### Vec<T> (20 methods)
push, pop, get, set, len, sort, reverse, clear, contains, index_of,
map, filter, reduce, for_each, any, all, sum, join

### String (13 methods)
len, substr, char_at, contains, starts_with, ends_with, index_of,
trim, to_upper, to_lower, replace, repeat
+ split via str_split_count/str_split_get

### HashMap
- `map_*` builtins: str→i32 (insert, get, contains, remove, free, len, key_at)
- `smap_*` builtins: str→str
- Generic `HashMap<K,V>`: str→i32, str→str, str→f64 via hmap_new + methods

### Memory Management
- alloc/free with OOM panic + null check
- defer (LIFO, function-scoped)
- Arena allocator (arena_new/alloc/free/reset)
- **GC**: mark-and-sweep garbage collector (gc_alloc/gc_collect/gc_stats/gc_bytes_allocated/gc_collection_count)
- **`new` keyword**: `new Type { fields }` → GC-managed heap allocation, returns `*Type`

### Concurrency (Go-style)
- **Goroutines**: `go fn(args)` — fire-and-forget dispatch to thread pool
- **Typed channels**: `chan<T>` with `.send(val)`, `.recv()`, `.close()` methods
- **`select`**: channel multiplexing (`select { v := ch.recv() => { ... } }`)
- Threads: thread_spawn/thread_join (pthreads)
- Channels: channel_new/send/recv/close (bounded blocking, i32)
- Thread pool: pool_new/submit/wait/free
- Async/await: async fn + await (deprecated — use `go` + channels)

### Null Safety
- **`nil` literal**: null pointer value, `ptr == nil` / `ptr != nil`
- **`?T` optional types**: `?*Point` = nullable pointer, compile-time field access prevention
- **`??` null coalescing**: `p ?? default` — unwrap optional or use default

### Error Handling
- **`error_new(msg)`**: create error with string message → returns error code
- **`error_message(code)`**: retrieve error message string
- **`?` operator**: `val := divide(10, 0)?;` — propagate errors
- Pattern matching on `enum Result { Ok(i32), Err(i32) }`

### Go-style Ergonomics
- **`var` keyword**: `var x = 5;` — readable mutable declaration (alternative to `:~=`)
- **`type` aliases**: `type Meters = f64;` (Go-style type definitions)
- **`for key, val in map`**: Go-style HashMap iteration
- **`dyn Trait` returns**: functions can return `dyn Trait` (auto-coercion)
- **`println` spaces**: automatic spaces between multiple arguments
- **Deprecation warnings**: `async/await` warns to use `go` + channels; operator overloading warns on non-numeric types

### Tensor API (22 operations)
tensor_zeros, tensor_ones, tensor_fill, tensor_rand, tensor_clone,
tensor_get, tensor_set, tensor_rows, tensor_cols,
tensor_add, tensor_sub, tensor_mul, tensor_scale,
tensor_matmul, tensor_transpose, tensor_dot, tensor_norm,
tensor_sum, tensor_max, tensor_min, tensor_print, tensor_free

### Builtins (115)
print/println, alloc/free/sizeof, math (sqrt/sin/cos/floor/ceil/pow/fabs/log/exp),
file I/O (read_file/write_file/remove_file/fopen/fclose/fread_byte),
conversion (int_to_str/str_to_int/float_to_str/str_to_float),
JSON (json_parse/get_str/get_int/get_float/get_bool/len/arr_get/free),
GC (gc_alloc/gc_collect/gc_stats/gc_bytes_allocated/gc_collection_count),
channels (chan_new, channel_new/send/recv/close),
errors (error_new/error_message),
timing (clock_ms/sleep_ms), exit

### Tooling
- **ny fmt**: Comment-preserving formatter (standalone + trailing) + compound assign reconstruction
- **ny test**: Per-test timing, compile error printing
- **ny check**: Type-check without codegen, timing per phase
- **ny repl**: Interactive with persistent definitions
- **ny-lsp**: Diagnostics, hover (semantic + param names), goto-def, completion, document symbols
- **ny pkg**: Git-based package manager with SHA pinning
- **VS Code extension**: Syntax highlighting + LSP client

### Error Handling
- codespan-reporting with line/col/arrows
- "Did you mean?" for variables, functions, struct fields, Vec/str methods
- Parser error recovery (multiple errors reported)
- Runtime stack traces on panic (debug builds)
- Exhaustiveness checking for match expressions

### Performance
- LLVM -O0 to -O3 optimization
- Release mode (-O2+): no bounds checks, no stack traces
- Ny wins or ties Go in ALL 7 benchmarks
- WASM target: ny build --target wasm32

## Reserved Keywords

fn, if, else, while, for, in, return, struct, break, continue, as, enum, match,
defer, pub, use, mod, trait, impl, loop, unsafe, extern, let, async, await, new, dyn, go, nil, select, type, var

## Roadmap

See [specs/ROADMAP.md](specs/ROADMAP.md) for the full strategic roadmap.
