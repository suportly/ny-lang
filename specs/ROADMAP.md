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
| 13 | AI/ML Compute | SIMD intrinsics (f32x4/f32x8), par_map/par_reduce | **Tensor\<T\> type**, **GPU compute (NVPTX)**, auto-vectorization |

---

## What's Next (prioritized)

### Tier 1 — Language Power (enables more real programs)

#### 15. Generic HashMap\<K,V\>
**Unlocks**: str→str, int→struct, any key/value combination
**Current**: HashMap is str→i32 only, backed by C runtime
**Target**: Generic HashMap with monomorphized key/value types
**Complexity**: Very High — requires codegen refactor for struct-valued maps

#### 16. Vec\<str\> and Vec\<struct\>
**Unlocks**: split() as method returning Vec\<str\>, collections of complex types
**Current**: Vec works for primitives (i32, f64, bool, i8) but not for str (16-byte struct)
**Target**: Vec of any type via elem_size-aware codegen
**Complexity**: High — struct elements need correct GEP and memcpy handling

#### 17. Trait Bounds Enforcement
**Current**: `fn max<T: Ord>(a: T, b: T) -> T` parses but `Ord` bound is ignored
**Target**: Type-check that `T` actually satisfies the bound at call sites
**Complexity**: Medium

### Tier 2 — Developer Experience

#### 18. LSP Multi-File Support
**Current**: LSP doesn't resolve `use "module.ny"` imports — false errors for imported symbols
**Target**: Run resolve_uses before semantic analysis in LSP
**Complexity**: Medium

#### 19. Operator Overloading via Traits
**Current**: +, -, *, / only work on primitives
**Target**: `impl Add for Point { fn add(self, other) -> Point }` → `p1 + p2`
**Complexity**: High

#### 20. Publish VS Code Extension
**Current**: Works locally, not on marketplace
**Target**: `vsce package` + publish to VS Code marketplace
**Dependency**: Node 20+ for vsce

### Tier 3 — The Differentiator

#### 21. Tensor\<T\> Type (First-Class)
```ny
a := Tensor<f32>::zeros(3, 3);
b := Tensor<f32>::ones(3, 3);
c := a + b;           // element-wise
d := a.matmul(b);     // matrix multiply
println(d.shape());   // (3, 3)
```
**Complexity**: Very High — new type, SIMD kernels, shape metadata

#### 22. GPU Compute via NVPTX
```ny
#[gpu]
fn vector_add(a: []f32, b: []f32, out: []f32) {
    idx := gpu::thread_id();
    out[idx] = a[idx] + b[idx];
}
```
**Complexity**: Very High — LLVM NVPTX backend, CUDA runtime wrapper

#### 23. Package Manager (ny pkg)
```bash
ny pkg init
ny pkg add math-extra
ny pkg build
```
**Complexity**: High — registry, dependency resolution, versioning

### Tier 4 — Ecosystem

#### 24. WASM Target
**Target**: `ny build --target wasm32` for browser/playground
**Complexity**: Medium — LLVM supports wasm, mainly linker changes

#### 25. Autograd / Automatic Differentiation
**Target**: `grad(loss_fn, params)` for ML training
**Complexity**: Very High — requires reverse-mode AD on the IR

---

## Performance Benchmarks

Verified on x86-64 Linux, median of 5 runs:

| Benchmark | C (gcc -O2) | Ny -O2 | Go | Ny vs C | Ny vs Go |
|-----------|-------------|--------|-----|---------|----------|
| fib(40) | 240ms | 407ms | 593ms | 1.7x | **1.5x faster** |
| matmul 256x256 | 19ms | 25ms | 40ms | 1.3x | **1.6x faster** |

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
| Tests | 116 (100 integration + 16 error) |
| Lines of Rust | ~23,000 |
| Runtime C files | 6 (hashmap, arena, channel, threadpool, string, json) |
| Builtins | 70+ |
| Vec\<T\> methods | 18 |
| String methods | 13 |
| CLI commands | 7 (build, run, check, test, fmt, repl, lsp) |
| Examples | 12 |
| Benchmarks | Ny is 1.5x faster than Go, 1.3-1.7x of C |
