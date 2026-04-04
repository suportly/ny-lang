# Ny Lang — Strategic Roadmap

**Updated**: 2026-04-04
**Goal**: "Go with algebraic types" — native-compiled, GC-managed, with interfaces + goroutines + channels + pattern matching
**Competitor Reference**: Go (concurrency model), Rust (type safety), Zig (simplicity), Mojo (AI/ML)
**Positioning**: Native-compiled language combining Go's concurrency (goroutines, channels, select) with Rust's type safety (enums, pattern matching, traits) and optional GC

---

## Implementation Status

### Completed Phases

| Phase | Feature | Status | Tests |
|-------|---------|--------|-------|
| 1-3 | Compiler MVP: scalars, functions, control flow, arrays, structs, pointers, strings, loops, operators, casting | **COMPLETE** | 30+ |
| 4 | Strings, Enums, Tuples, Match (exhaustiveness checking) | **COMPLETE** | 10+ |
| 5 | Heap Memory (alloc/free/sizeof), Defer (LIFO), Arena allocator | **COMPLETE** | 5+ |
| 6 | Tagged Unions (data-carrying enums), ? operator, if let, while let, loop | **COMPLETE** | 8+ |
| 7 | Module System (use "file.ny"), pub keyword, extern C FFI | **COMPLETE** | 3+ |
| 8 | Generic functions + structs + enums (monomorphization), Traits, Impl blocks | **COMPLETE** | 10+ |
| 9 | Slices []T, Vec\<T\> (18 methods), HashMap (str→i32) | **COMPLETE** | 15+ |
| 10 | File I/O (read_file/write_file/fopen/fclose/fread_byte), read_line, remove_file | **COMPLETE** | 3+ |
| 11 | Closures (capturing), Lambdas, HOFs, map/filter/reduce/for_each/any/all | **COMPLETE** | 8+ |
| 12 | Threads (pthreads), Channels (bounded blocking), Thread pool, sleep_ms | **COMPLETE** | 7+ |
| 14 | ny build/run/check/test/fmt/repl, LSP (6 capabilities), VS Code extension | **COMPLETE** | 5+ |

### Additional Features (beyond original roadmap)

| Feature | Description |
|---------|-------------|
| JSON parser | json_parse/get_str/get_int/get_float/get_bool/len/arr_get/free |
| Math builtins | sqrt, sin, cos, floor, ceil, pow, fabs, log, exp |
| Float conversion | float_to_str, str_to_float |
| String methods (13) | len, substr, char_at, contains, starts_with, ends_with, index_of, trim, to_upper, to_lower, replace, repeat, split |
| clock_ms | Monotonic timer for benchmarking |
| Runtime stack traces | Call chain printed on panic (bounds check, OOM) |
| "Did you mean?" | Typo suggestions for variables, functions, struct fields, Vec/str methods |
| Parser error recovery | Multiple errors reported, not just the first |
| Formatter | Comment preservation (standalone + trailing), compound assignment reconstruction |
| f-string interpolation | f"value is {expr}" |
| SIMD intrinsics | f32x4, f32x8 splat/load/store/reduce + arithmetic |

### Partially Implemented

| Phase | Feature | What's Done | What's Missing |
|-------|---------|-------------|----------------|
| 13 | AI/ML Compute | SIMD intrinsics, par_map/par_reduce, **Tensor (22 ops)** | GPU compute (NVPTX), auto-vectorization |

### Recently Completed (beyond original phases)

| Phase | Feature | Status |
|-------|---------|--------|
| 15 | Generic HashMap\<K,V\> (str→i32, str→str, str→f64) | **COMPLETE** |
| 16 | Vec\<str\> and Vec\<struct\> | **COMPLETE** |
| 17 | Trait Bounds Enforcement | **COMPLETE** |
| 18 | LSP Multi-File Support | **COMPLETE** |
| 19 | Operator Overloading (+, -, *, / for structs) | **COMPLETE** |
| 20 | Package Manager (ny pkg init/add/build/remove/list) | **COMPLETE** |
| 21 | Tensor\<f64\> (22 operations) | **COMPLETE** |
| 22 | WASM Target (ny build --target wasm32) | **COMPLETE** |
| 23 | Release Mode (-O2+ skips bounds checks + traces) | **COMPLETE** |
| 24 | Vec.join(sep) for string building | **COMPLETE** |
| 25 | Async/Await (async fn + await + thread pool dispatch) | **COMPLETE** |
| 26 | Garbage Collector (mark-and-sweep, `new` keyword, gc_alloc/gc_collect/gc_stats) | **COMPLETE** |
| 27 | Dynamic Dispatch (`dyn Trait`, vtables, fat pointers, thunk-based dispatch) | **COMPLETE** |
| 28 | Typed Channels (`chan<T>`, `.send()/.recv()/.close()`, generic runtime) | **COMPLETE** |
| 29 | Goroutines (`go fn(args)` — fire-and-forget thread pool dispatch) | **COMPLETE** |
| 30 | `nil` literal (null pointer value + pointer comparison) | **COMPLETE** |
| 31 | `select` statement (channel multiplexing with polling try_recv) | **COMPLETE** |
| 32 | Functions returning `dyn Trait` + `for key, val in map` iteration | **COMPLETE** |
| 33 | `type` aliases + `println` spaces between args (Go-style) | **COMPLETE** |
| 34 | Error handling with string messages (`error_new`/`error_message` + `?` operator) | **COMPLETE** |
| 35 | `?T` optional types + `??` null coalescing + compile-time null safety | **COMPLETE** |
| 36 | `var` keyword + `async/await` deprecation warnings + operator overloading warnings | **COMPLETE** |

---

## What's Next (prioritized)

### Tier 1 — Language Completeness

- **Standard library modules**: `io`, `os`, `net`, `fmt` as importable packages
- **Error trait**: formalize `trait Error { fn message(self) -> str; }` as a builtin trait
- **Green threads**: M:N scheduler replacing OS thread pool for `go`
- **Generic enums**: `enum Result<T, E> { Ok(T), Err(E) }` with full str payload support

### Tier 2 — Performance

- **GPU Compute (NVPTX)**: `#[gpu] fn` compiled to CUDA kernels
- **Autograd**: `grad(loss_fn, params)` for ML training
- **Escape analysis**: stack vs heap decision for `new` allocations

---

## Performance Benchmarks

Ny wins or ties Go in ALL 7 benchmarks (at -O2, median of 3 runs):

| Benchmark | Ny -O2 | C -O2 | Go | Ny vs Go |
|-----------|--------|-------|-----|----------|
| N-Body (physics) | 50ms | 39ms | 50ms | **tied** |
| Spectral Norm | 269ms | 235ms | 280ms | **tied** |
| Fibonacci fib(40) | 375ms | 250ms | 654ms | **1.7x faster** |
| Ackermann(3,12) | 2300ms | 700ms | 4100ms | **1.8x faster** |
| Binary Trees | 108ms | 148ms | 236ms | **2.2x faster** |
| Matrix Multiply 256 | 25ms | 19ms | 40ms | **1.6x faster** |
| Sieve 10M | 112ms | 79ms | 86ms | **1.3x** |

---

## Non-Goals (Explicitly Out of Scope)

- **Web backend framework** — Go and Rust own this
- **Dynamic typing** — Ny is statically typed
- **OOP (classes, inheritance)** — Structs + traits + dyn Trait only
- **Regex engine** — Use via FFI (pcre2)
- **HTTP server** — Use via FFI (libcurl)

---

## Metrics

| Metric | Value |
|--------|-------|
| Tests | 142 |
| Lines of Rust | ~23,000 |
| Runtime C files | 12 (hashmap, hashmap_generic, arena, channel, threadpool, string, json, tensor, future, gc, chan, error) |
| Builtins | 115 |
| Vec\<T\> methods | 20 (push/pop/get/set/len/sort/reverse/clear/contains/index_of/map/filter/reduce/for_each/any/all/sum/join) |
| String methods | 13 |
| Tensor operations | 22 |
| HashMap types | Generic HashMap\<K,V\> + map_* + smap_* |
| CLI commands | 12 (build/run/check/test/fmt/repl/lsp + pkg init/add/build/remove/list) |
| Examples | 15 |
| Benchmarks | 7 (Ny wins/ties Go in all) |
| Targets | native x86-64 + wasm32 |
| Benchmarks | Ny is 1.5x faster than Go, 1.3-1.7x of C |
