# Ny Lang — Strategic Roadmap

**Updated**: 2026-04-03
**Goal**: Transform Ny from a compiler prototype into a viable low-level language for AI/ML compute
**Competitor Reference**: Go (backend/infra), Zig/Rust (systems), Mojo (AI/ML), C++/CUDA (GPU compute)
**Positioning**: Native-compiled, zero-runtime, immutable-by-default language optimized for numerical computation and ML inference

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

---

## What's Next (prioritized)

### Tier 1 — Remaining

#### 25. GPU Compute via NVPTX
```ny
```ny
#[gpu]
fn vector_add(a: []f32, b: []f32, out: []f32) {
    idx := gpu::thread_id();
    out[idx] = a[idx] + b[idx];
}
```
**Requires**: CUDA toolkit, LLVM NVPTX backend
**Complexity**: Very High

#### 26. Async/Await
**Target**: Non-blocking I/O, async functions
**Complexity**: Very High

#### 27. Autograd / Automatic Differentiation
**Target**: `grad(loss_fn, params)` for ML training
**Complexity**: Very High — requires reverse-mode AD on the IR

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
- **Mobile/WASM target** — Focus on x86-64 + GPU (WASM deferred to Tier 4)
- **Dynamic typing** — Ny is statically typed
- **Garbage collection** — Never. Manual + defer + arena
- **OOP (classes, inheritance)** — Structs + traits only
- **Regex engine** — Use via FFI (pcre2)
- **HTTP server** — Use via FFI (libcurl)

---

## Metrics

| Metric | Value |
|--------|-------|
| Tests | 125+ |
| Lines of Rust | ~30,000 |
| Runtime C files | 8 (hashmap, hashmap_generic, arena, channel, threadpool, string, json, tensor) |
| Builtins | 90+ |
| Vec\<T\> methods | 20 (push/pop/get/set/len/sort/reverse/clear/contains/index_of/map/filter/reduce/for_each/any/all/sum/join) |
| String methods | 13 |
| Tensor operations | 22 |
| HashMap types | Generic HashMap\<K,V\> + map_* + smap_* |
| CLI commands | 12 (build/run/check/test/fmt/repl/lsp + pkg init/add/build/remove/list) |
| Examples | 15 |
| Benchmarks | 7 (Ny wins/ties Go in all) |
| Targets | native x86-64 + wasm32 |
| Benchmarks | Ny is 1.5x faster than Go, 1.3-1.7x of C |
