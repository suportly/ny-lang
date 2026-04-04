# Changelog

## v1.0.0 (2026-04-04)

First stable release. **"Go's concurrency + Rust's type safety."**

### Language Features (41 phases complete)

**Core**
- 14 scalar types, arrays, slices, tuples, structs, enums, pointers
- Generics with monomorphization
- Closures with variable capture
- Pattern matching with exhaustiveness checking
- f-string interpolation

**Type System**
- Traits + `impl Trait for Type` + trait bounds
- `dyn Trait` — dynamic dispatch via vtables and fat pointers
- `interface` keyword (alias for `trait`, Go-style naming)
- `?T` optional types with compile-time null safety
- `??` null coalescing operator
- `type` aliases (`type Meters = f64;`)
- Operator overloading (with warnings on non-numeric types)

**Memory Management**
- Manual `alloc`/`free` with `defer` (LIFO cleanup)
- Arena allocator
- Mark-and-sweep garbage collector
- `new` keyword for GC-managed heap allocation
- `nil` literal with pointer comparison

**Concurrency (Go-style)**
- `go fn(args)` — goroutines dispatched to thread pool
- `chan<T>` — typed channels with `.send()`, `.recv()`, `.close()`
- `select` — channel multiplexing
- Auto-wait at program exit (goroutines complete before main returns)
- Thread pool, raw threads, and blocking channels

**Error Handling**
- Tagged union `Result { Ok(T), Err(str) }` with mixed-size payloads
- `?` operator for error propagation
- `error_new(msg)` / `error_message(code)` / `error_trace(code)`
- Stack traces captured at error creation (debug builds)

**Control Flow**
- `if let val = optional` — safe unwrap for `?T`
- `for key, value in map` — Go-style HashMap iteration
- `var x = 5;` — readable mutable declaration
- `while let`, `loop`, `break`, `continue`

### Collections
- `Vec<T>` — 20 methods (push, pop, sort, map, filter, reduce, ...)
- `HashMap<K,V>` — generic with iteration
- String — 13 methods + split
- `Tensor<f64>` — 22 matrix operations
- SIMD intrinsics (f32x4, f32x8)

### Tooling
- `ny build` / `ny run` / `ny check` / `ny test`
- `ny fmt` — comment-preserving formatter
- `ny repl` — interactive mode
- `ny-lsp` — Language Server (diagnostics, hover, goto-def, completion, symbols)
- `ny pkg` — git-based package manager with SHA pinning
- VS Code extension with syntax highlighting
- WASM target (`ny build --target wasm32`)
- 7 benchmarks (Ny wins or ties Go in all)

### Metrics
- 147 tests (131 integration + 16 negative)
- ~23,000 lines of Rust (compiler)
- 12 runtime C files
- 115+ builtins
- 32 keywords
- 21 example programs

### Known Limitations
See [docs/LIMITATIONS.md](docs/LIMITATIONS.md) for full details.
- `go` uses fixed-size OS thread pool, not green threads
- GC is mark-and-sweep stop-the-world
- Error handling uses global table, not typed objects
- `async`/`await` is deprecated (use `go` + channels)
