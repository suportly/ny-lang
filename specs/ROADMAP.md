# Ny Lang — Strategic Roadmap

**Created**: 2026-04-01
**Goal**: Transform Ny from a compiler prototype into a viable low-level language for AI/ML compute
**Competitor Reference**: Go (backend/infra), Zig/Rust (systems), Mojo (AI/ML), C++/CUDA (GPU compute)
**Positioning**: Native-compiled, zero-runtime, immutable-by-default language optimized for numerical computation and ML inference

---

## Current State (Phases 1–3 Complete)

| Capability | Status |
|---|---|
| Scalar types (14 numeric + bool) | Done |
| Functions, recursion, control flow | Done |
| Arrays (fixed-size), structs, methods | Done |
| Pointers, auto-deref | Done |
| Strings (literals, print/println) | Done |
| For/while loops, break/continue | Done |
| Type inference (`:=`, `:~=`) | Done |
| Bitwise, compound assignment, casting | Done |
| LLVM backend with O0–O3 | Done |

**What's missing for real programs**: heap memory, error handling, modules, I/O, concurrency, generics, dynamic data structures, tooling.

---

## Dependency Graph

```
Phase 4: Strings, Enums, Tuples ─────────────────────────┐
    │                                                      │
Phase 5: Heap Memory & Defer ─────────────────────────┐   │
    │                                                  │   │
Phase 6: Error Handling (Result/Option) ──────────┐    │   │
    │                                              │   │   │
Phase 7: Module System & Imports ─────────────┐    │   │   │
    │                                          │   │   │   │
    ├──── Phase 8: Generics & Traits ─────┐    │   │   │   │
    │         │                            │   │   │   │   │
    │     Phase 9: Slices & Collections    │   │   │   │   │
    │         │                            │   │   │   │   │
    │     Phase 11: Closures & HOFs        │   │   │   │   │
    │                                      │   │   │   │   │
    ├──── Phase 10: I/O & Syscalls ────────┤   │   │   │   │
    │                                      │   │   │   │   │
    ├──── Phase 12: Concurrency ───────────┤   │   │   │   │
    │         │                            │   │   │   │   │
    └──── Phase 13: AI/ML Compute ─────────┘   │   │   │   │
              │                                │   │   │   │
          Phase 14: Tooling & Ecosystem ───────┘───┘───┘───┘
```

**Critical path**: 4 → 5 → 6 → 7 → 8 → 9 (enables real programs)
**AI/ML path**: 8 + 12 → 13 (enables the differentiator)

---

## Phase 4: Strings, Enums & Tuples *(specced, next to implement)*

**Branch**: `004-strings-enums-tuples`
**Spec**: [specs/004-strings-enums-tuples/spec.md](004-strings-enums-tuples/spec.md)
**Unlocks**: Text processing, state machines, multi-return values, pattern matching

### Summary
- String operations: `.len()`, `+` (concat), `==`/`!=`, `.substr()`
- Enums: named variants (no payloads), `EnumName::Variant`, equality
- Match expressions: exhaustiveness checking, wildcard `_`, block arms
- Tuples: `(T1, T2)`, `.0`/`.1` access, destructuring, multi-return

### Why before Phase 5
Enums + match are prerequisites for `Result<T, E>` and `Option<T>` in Phase 6. Tuples enable Go-style multi-return `(val, err)` as a stepping stone. String concat introduces the first `malloc` call (heap), which Phase 5 formalizes.

---

## Phase 5: Heap Memory & Defer

**Unlocks**: Dynamic data, string ownership, resource cleanup
**Depends on**: Phase 4 (strings already use malloc)
**Estimated complexity**: High — first major runtime concept

### Core Features

#### 5.1 — Heap Allocation
```ny
// Raw allocation (low-level, explicit)
p := alloc(i32);        // allocates sizeof(i32) on heap, returns *i32
*p = 42;
free(p);                // manual deallocation

// Array allocation (dynamic size)
buf := alloc([1024]u8);  // heap-allocated array
free(buf);
```

#### 5.2 — `defer` Statement
```ny
fn read_data() -> i32 {
    buf := alloc([1024]u8);
    defer free(buf);         // executes when scope exits

    // ... use buf ...
    return process(buf);     // free(buf) runs after return
}
```

#### 5.3 — Owned Strings
```ny
s1 : str = "hello";          // stack (string literal, read-only)
s2 := s1 + " world";         // heap-allocated (from Phase 4 concat)
defer free_str(s2);           // explicit cleanup (no GC)
```

### Design Decisions
- **No GC, no borrow checker** — manual memory management with `defer` as the ergonomic safety net
- `alloc`/`free` are compiler builtins wrapping `malloc`/`free` from libc
- `defer` uses LIFO order (last defer runs first), executes on all exit paths (return, break, end of block)
- Dangling pointers are undefined behavior (no runtime checks) — future phases may add optional sanitizers
- This is intentionally simpler than Rust (no lifetimes) and more explicit than Go (no GC). The tradeoff is programmer discipline for zero-cost abstraction

### Implementation Strategy
- Add `defer` to AST as `Stmt::Defer { body: Expr }`
- Codegen: emit defer'd expressions at every scope exit point (return, break, block end)
- `alloc(T)` → `malloc(sizeof(T))` + bitcast to `*T`
- `free(p)` → `free(bitcast p to i8*)`
- Refactor Phase 4 string concat to use this `alloc`/`free` infrastructure

### Success Criteria
- A program that allocates, uses, and frees heap memory without leaks (verified by valgrind)
- `defer` correctly executes in LIFO order on all exit paths
- String concat uses the new alloc infrastructure
- No regressions on Phase 1–4 tests

---

## Phase 6: Error Handling

**Unlocks**: Robust programs, failure recovery, the `?` operator
**Depends on**: Phase 4 (enums + tuples), Phase 5 (heap for error messages)
**Estimated complexity**: Medium — builds on existing enum/match infrastructure

### Core Features

#### 6.1 — Result and Option as Built-in Enums
```ny
// Compiler-provided generic-like types (hardcoded until Phase 8 generics)
// Result is a tagged union: Ok(T) | Err(E)
// Option is a tagged union: Some(T) | None

fn divide(a: i32, b: i32) -> Result_i32 {
    if b == 0 {
        return Err("division by zero");
    }
    return Ok(a / b);
}

fn find(arr: [10]i32, target: i32) -> Option_i32 {
    for i in 0..10 {
        if arr[i] == target {
            return Some(arr[i]);
        }
    }
    return None;
}
```

#### 6.2 — Data-Carrying Enum Variants (Tagged Unions)
```ny
enum Shape {
    Circle(f64),                    // radius
    Rectangle(f64, f64),            // width, height
    Triangle(f64, f64, f64),        // sides
}

fn area(s: Shape) -> f64 {
    match s {
        Shape::Circle(r) => 3.14159 * r * r,
        Shape::Rectangle(w, h) => w * h,
        Shape::Triangle(a, b, c) => {
            sp := (a + b + c) / 2.0;
            // Heron's formula (sqrt not yet available, placeholder)
            return sp;
        },
    }
}
```

#### 6.3 — The `?` Operator (Error Propagation)
```ny
fn process() -> Result_i32 {
    val := divide(10, 0)?;   // if Err, return Err immediately
    return Ok(val + 1);
}
```

### Design Decisions
- **Pre-generics approach**: `Result_i32`, `Option_i32`, `Result_str` etc. are compiler-generated monomorphized types. When generics arrive (Phase 8), these become `Result<i32, str>`, `Option<i32>`
- Tagged unions store a discriminant + max-sized payload (C-style union layout)
- `?` desugars to `match expr { Ok(v) => v, Err(e) => return Err(e) }`
- `None` and `Err` patterns work in match arms with variable binding
- No exceptions, no panics — all errors are values

### Implementation Strategy
- Extend `NyType::Enum` to support data payloads: `Enum { name, variants: Vec<(String, Vec<NyType>)> }`
- Tagged union codegen: `{ tag: i8, payload: [max_size x i8] }` with bitcasts for access
- Pattern matching in `match` extracts payload with variable binding
- `?` operator: new `Expr::Try` AST node, desugars in semantic phase or codegen

---

## Phase 7: Module System & Imports

**Unlocks**: Multi-file programs, code organization, separate compilation
**Depends on**: Phase 6 (error types shared across modules)
**Estimated complexity**: High — affects entire compilation pipeline

### Core Features

#### 7.1 — Module Declaration and Imports
```ny
// math.ny
mod math;

pub fn sqrt(x: f64) -> f64 { ... }
pub fn abs(x: i32) -> i32 { ... }
fn internal_helper() -> i32 { ... }  // private by default

// main.ny
use math;
use math::sqrt;      // import specific symbol

fn main() -> i32 {
    r := math::abs(-5);
    s := sqrt(2.0);
    return 0;
}
```

#### 7.2 — Visibility
```ny
pub fn visible() -> i32 { ... }    // accessible from other modules
fn private() -> i32 { ... }        // module-private (default)
pub struct Point { ... }            // public struct
```

#### 7.3 — File Resolution
```
project/
├── main.ny           # entry point
├── math.ny           # mod math
├── utils/
│   ├── mod.ny        # mod utils (directory module)
│   └── strings.ny    # mod utils::strings
```

### Design Decisions
- **One file = one module** (like Go, unlike Rust's inline `mod {}`)
- Private by default, `pub` keyword for exports (like Rust)
- No circular dependencies — enforced at compile time
- Compilation order: topological sort of dependency graph
- Each module compiles to its own LLVM module, then linked together
- No package manager yet — modules are local files only. Third-party packages deferred to Phase 14

### Implementation Strategy
- New `src/module/` module: module graph, dependency resolution, file discovery
- Extend `lib.rs` pipeline: resolve modules → compile each → link
- Add `pub` keyword to lexer/parser
- Semantic analysis: cross-module name resolution, visibility checking
- Codegen: one LLVM module per `.ny` file, link with `llvm-link` or linker

---

## Phase 8: Generics & Traits

**Unlocks**: Reusable abstractions, generic collections, operator overloading
**Depends on**: Phase 7 (modules, for stdlib organization)
**Estimated complexity**: Very High — the hardest phase, core type system change

### Core Features

#### 8.1 — Generic Functions
```ny
fn max<T: Ord>(a: T, b: T) -> T {
    if a > b { return a; }
    return b;
}

fn swap<T>(a: *T, b: *T) {
    tmp := *a;
    *a = *b;
    *b = tmp;
}
```

#### 8.2 — Generic Structs
```ny
struct Vec<T> {
    data: *T,
    len: i64,
    cap: i64,
}

struct Pair<A, B> {
    first: A,
    second: B,
}
```

#### 8.3 — Traits (Interfaces)
```ny
trait Numeric {
    fn zero() -> Self;
    fn add(self, other: Self) -> Self;
    fn mul(self, other: Self) -> Self;
}

// Implement for i32
impl Numeric for i32 {
    fn zero() -> i32 = 0;
    fn add(self, other: i32) -> i32 = self + other;
    fn mul(self, other: i32) -> i32 = self * other;
}

// Use in generic context
fn dot<T: Numeric>(a: [N]T, b: [N]T, n: i64) -> T {
    sum :~ T = T::zero();
    for i in 0..n {
        sum = sum.add(a[i].mul(b[i]));
    }
    return sum;
}
```

#### 8.4 — Operator Overloading via Traits
```ny
trait Add {
    fn add(self, other: Self) -> Self;  // maps to +
}

trait Index {
    type Output;
    fn index(self, idx: i64) -> Output;  // maps to []
}
```

### Design Decisions
- **Monomorphization** (like Rust/C++, unlike Go interfaces) — zero-cost at runtime, code bloat tradeoff
- Traits are the ONLY way to constrain generics — no structural typing
- `impl Trait for Type` syntax (Rust-style) — explicit, not implicit
- No trait objects / dynamic dispatch in this phase (deferred)
- `Self` keyword in traits refers to the implementing type
- Result<T, E> and Option<T> become proper generic types, replacing Phase 6 monomorphized versions

### Implementation Strategy
- Extend parser: `<T>` syntax on fn/struct, `trait`/`impl` keywords, `where` clauses
- New `src/generics/` module: monomorphization engine
- Type checker: trait bounds checking, associated type resolution
- Codegen: generate specialized LLVM functions per concrete type instantiation
- Migrate `Result_i32` → `Result<i32, str>`, `Option_i32` → `Option<i32>`

---

## Phase 9: Slices & Dynamic Collections

**Unlocks**: Dynamic arrays, hashmaps, real data processing
**Depends on**: Phase 5 (heap), Phase 8 (generics)
**Estimated complexity**: Medium — mostly library code on top of generics + heap

### Core Features

#### 9.1 — Slices
```ny
fn sum(data: []i32) -> i32 {       // []T = slice (ptr + len, no ownership)
    total :~ i32 = 0;
    for i in 0..data.len() {
        total += data[i];
    }
    return total;
}

arr : [5]i32 = [1, 2, 3, 4, 5];
s := arr[1..4];                     // slice of arr: [2, 3, 4]
println(sum(s));                    // 9
```

#### 9.2 — Vec<T> (Dynamic Array)
```ny
use std::vec;

v :~ Vec<i32> = Vec::new();
v.push(1);
v.push(2);
v.push(3);
println(v.len());       // 3
println(v[0]);           // 1
v.pop();                 // removes 3

// From array
v2 := Vec::from([1, 2, 3, 4, 5]);
```

#### 9.3 — HashMap<K, V>
```ny
use std::map;

m :~ HashMap<str, i32> = HashMap::new();
m.insert("one", 1);
m.insert("two", 2);

match m.get("one") {
    Some(val) => println(val),   // 1
    None => println("not found"),
}
```

### Design Decisions
- Slices are `{ ptr: *T, len: i64 }` — borrowed view, no ownership
- `Vec<T>` owns its data, grows via `realloc`, implements `Drop` trait (auto-free)
- These are the first "stdlib" types — live in `std::vec`, `std::map` modules
- Bounds checking on all slice/vec access (debug mode), optional elision with `unsafe` (future)

---

## Phase 10: I/O & System Calls

**Unlocks**: File reading/writing, stdin/stdout, network (basic), real-world programs
**Depends on**: Phase 5 (heap buffers), Phase 6 (Result for errors), Phase 9 (Vec for buffers)
**Estimated complexity**: Medium — mostly FFI to libc

### Core Features

#### 10.1 — File I/O
```ny
use std::fs;

fn main() -> i32 {
    // Read entire file
    match fs::read_string("input.txt") {
        Ok(content) => println(content),
        Err(e) => { println(e); return 1; },
    }

    // Write to file
    fs::write("output.txt", "hello world")?;

    return 0;
}
```

#### 10.2 — Stdin/Stdout Streams
```ny
use std::io;

fn main() -> i32 {
    line := io::read_line()?;
    io::write("You said: ")?;
    io::write_line(line)?;
    return 0;
}
```

#### 10.3 — Process & Environment
```ny
use std::env;
use std::process;

fn main() -> i32 {
    args := env::args();          // Vec<str>
    for i in 0..args.len() {
        println(args[i]);
    }

    process::exit(1);            // immediate exit
}
```

### Implementation Strategy
- `std::fs` and `std::io` wrap libc: `open`, `read`, `write`, `close`, `fstat`
- All I/O returns `Result<T, str>` — errors are errno descriptions
- Buffered I/O via internal `Vec<u8>` buffers
- No async I/O — blocking only (async deferred to Phase 12)

---

## Phase 11: Closures & Higher-Order Functions

**Unlocks**: Functional patterns, callbacks, iterators
**Depends on**: Phase 5 (heap for closure environments), Phase 8 (generics for `Fn` trait)
**Estimated complexity**: High — captures, lifetime of captured variables

### Core Features

#### 11.1 — Lambda Syntax
```ny
add := |a: i32, b: i32| -> i32 { a + b };
println(add(2, 3));   // 5

// Type inference for parameters when passed to generic functions
numbers.map(|x| x * 2);
```

#### 11.2 — Higher-Order Functions
```ny
fn apply<T>(f: fn(T) -> T, x: T) -> T {
    return f(x);
}

fn map<T, U>(arr: []T, f: fn(T) -> U) -> Vec<U> {
    result :~ Vec<U> = Vec::new();
    for i in 0..arr.len() {
        result.push(f(arr[i]));
    }
    return result;
}
```

#### 11.3 — Closures (Capturing Environment)
```ny
fn make_adder(n: i32) -> fn(i32) -> i32 {
    return |x: i32| -> i32 { x + n };   // captures `n`
}

add5 := make_adder(5);
println(add5(10));     // 15
```

### Design Decisions
- Closures that don't capture are thin function pointers (zero-cost)
- Closures that capture use heap-allocated environment struct (like C++ `std::function`)
- Capture by value (copy) by default — capture by reference with explicit `&`
- `fn(T) -> U` is the function pointer type, `Fn(T) -> U` is the closure trait (Phase 8)

---

## Phase 12: Concurrency

**Unlocks**: Parallel computation, async I/O, multi-core utilization
**Depends on**: Phase 5 (heap), Phase 8 (generics), Phase 10 (I/O)
**Estimated complexity**: Very High — memory model, synchronization primitives

### Core Features

#### 12.1 — Threads
```ny
use std::thread;

fn compute(id: i32) -> i32 {
    // heavy computation
    sum :~ i32 = 0;
    for i in 0..1000000 {
        sum += i;
    }
    return sum;
}

fn main() -> i32 {
    t1 := thread::spawn(|| compute(1));
    t2 := thread::spawn(|| compute(2));

    r1 := t1.join()?;
    r2 := t2.join()?;

    println(r1 + r2);
    return 0;
}
```

#### 12.2 — Channels (CSP-style, like Go)
```ny
use std::channel;

fn producer(ch: Channel<i32>) {
    for i in 0..100 {
        ch.send(i);
    }
    ch.close();
}

fn main() -> i32 {
    ch := channel::new<i32>(16);   // buffered channel, cap 16

    thread::spawn(|| producer(ch));

    // Receive until closed
    loop {
        match ch.recv() {
            Some(val) => println(val),
            None => break,
        }
    }
    return 0;
}
```

#### 12.3 — Atomics & Mutex
```ny
use std::sync;

counter :~ Mutex<i32> = Mutex::new(0);

fn increment() {
    lock := counter.lock();
    *lock += 1;
}   // auto-unlock via defer

// Atomic operations for lock-free code
use std::atomic;
a :~ Atomic<i64> = Atomic::new(0);
a.fetch_add(1);
```

### Design Decisions
- **Threads are OS threads** (like Rust, not goroutines) — predictable performance for compute
- Channels are typed and optionally buffered (CSP model inspired by Go)
- No data race protection by default (unlike Rust's `Send`/`Sync`) — trade safety for simplicity
- Mutex + atomics wrap pthreads/libc primitives
- Future optimization: work-stealing thread pool for parallel iterators

### Why not goroutines
Go's goroutines are optimized for I/O-heavy backend workloads (thousands of lightweight tasks). Ny targets compute-heavy ML workloads where you want 1 thread per core doing matrix math, not 10K tasks waiting on network. OS threads + explicit parallelism is the right model for numerical compute.

---

## Phase 13: AI/ML Compute Primitives *(the differentiator)*

**Unlocks**: The reason Ny Lang exists — native ML inference, numerical compute, GPU acceleration
**Depends on**: Phase 8 (generics), Phase 9 (slices/vec), Phase 12 (threads)
**Estimated complexity**: Very High — novel language features, hardware interaction

### Core Features

#### 13.1 — SIMD Intrinsics
```ny
use std::simd;

// Explicit SIMD types
fn dot_simd(a: []f32, b: []f32, n: i64) -> f32 {
    sum :~ f32x8 = f32x8::zero();       // 8-wide SIMD register
    i :~ i64 = 0;
    while i + 8 <= n {
        va := f32x8::load(a, i);         // load 8 floats
        vb := f32x8::load(b, i);
        sum = sum + va * vb;              // fused multiply-add
        i += 8;
    }
    result :~ f32 = sum.reduce_add();    // horizontal sum
    // Handle remainder
    while i < n {
        result += a[i] * b[i];
        i += 1;
    }
    return result;
}
```

#### 13.2 — Tensor Type (First-Class)
```ny
use std::tensor;

fn main() -> i32 {
    // 2D tensor (matrix)
    a := Tensor<f32>::zeros(3, 3);
    b := Tensor<f32>::ones(3, 3);

    // Matrix operations
    c := a + b;                    // element-wise add
    d := a.matmul(b);              // matrix multiply
    e := d.transpose();

    // Slicing
    row := d[0, :];                // first row
    col := d[:, 1];                // second column
    sub := d[0..2, 0..2];         // 2x2 submatrix

    println(d.shape());            // (3, 3)
    return 0;
}
```

#### 13.3 — Parallel Iterators
```ny
use std::par;

// Automatic parallelization over slices
fn normalize(data: []f32) -> Vec<f32> {
    total := data.par_reduce(0.0, |acc, x| acc + x);
    mean := total / data.len() as f32;
    return data.par_map(|x| x - mean);
}
```

#### 13.4 — GPU Compute (Stretch Goal)
```ny
use std::gpu;

#[gpu]
fn vector_add(a: []f32, b: []f32, out: []f32) {
    idx := gpu::thread_id();
    if idx < a.len() {
        out[idx] = a[idx] + b[idx];
    }
}

fn main() -> i32 {
    a := Tensor<f32>::rand(1000000);
    b := Tensor<f32>::rand(1000000);
    out := Tensor<f32>::zeros(1000000);

    gpu::launch(vector_add, a, b, out, threads: 1024, blocks: 1024);
    gpu::sync();

    println(out[0]);
    return 0;
}
```

### Design Decisions
- SIMD types map directly to LLVM vector types (`<8 x float>`) — zero abstraction cost
- Tensors are a library type built on `Vec<T>` + shape metadata + SIMD kernels
- `par_map`/`par_reduce` use the thread pool from Phase 12
- GPU: compile `#[gpu]` functions to PTX via LLVM's NVPTX backend (same LLVM infrastructure)
- Auto-vectorization hints via `#[simd]` attribute on loops
- No automatic differentiation in this phase — deferred to a future "autograd" phase

### Why this matters
This is where Ny stops being "another systems language" and becomes a tool people actually want. Today, ML inference in production means: Python calls C++ calls CUDA. Ny's pitch: **one language from algorithm to metal**. Write the matrix multiply, the inference loop, and the data pipeline in the same language, compiled to native + GPU with one toolchain.

---

## Phase 14: Tooling & Ecosystem

**Unlocks**: Developer adoption, productive development experience
**Depends on**: Phase 7 (modules, for package resolution), all other phases for full coverage
**Can be developed in parallel with language phases**

### Core Features

#### 14.1 — LSP (Language Server Protocol)
- Syntax highlighting, go-to-definition, find references
- Type-on-hover, inline error diagnostics
- Auto-completion for struct fields, enum variants, module exports
- Built on the existing lexer/parser/semantic infrastructure

#### 14.2 — Formatter (`ny fmt`)
- Opinionated, zero-config (like `gofmt`)
- Consistent indentation, brace style, line length
- Integrated into CI/pre-commit hooks

#### 14.3 — Package Manager (`ny pkg`)
```bash
ny pkg init                    # create ny.toml
ny pkg add math-extra          # add dependency
ny pkg build                   # build with deps
ny pkg publish                 # publish to registry
```

#### 14.4 — Test Framework
```ny
#[test]
fn test_addition() {
    assert_eq(1 + 1, 2);
    assert(true);
}
```
```bash
ny test                        # run all tests
ny test math                   # run tests in math module
```

#### 14.5 — Benchmarking
```ny
#[bench]
fn bench_matmul() {
    a := Tensor<f32>::rand(100, 100);
    b := Tensor<f32>::rand(100, 100);
    _ := a.matmul(b);
}
```
```bash
ny bench                       # run all benchmarks with statistics
```

### Development Strategy
- LSP can start after Phase 7 (modules) — incrementally add features
- Formatter can start after Phase 4 — only needs parser
- Package manager needs Phase 7 (modules) as foundation
- Test framework needs Phase 7 (modules) + attributes (new syntax)

---

## Timeline & Prioritization

### Tier 1 — Foundation (make real programs possible)

| Phase | Name | Estimated effort | Key unlock |
|---|---|---|---|
| 4 | Strings, Enums, Tuples | 1 sprint | Pattern matching, multi-return |
| 5 | Heap Memory & Defer | 1 sprint | Dynamic data, resource cleanup |
| 6 | Error Handling | 1 sprint | Robust programs, `?` operator |
| 7 | Module System | 2 sprints | Multi-file programs, stdlib |

**Milestone**: After Phase 7, Ny can write non-trivial single-purpose programs (CLI tools, data processors). First "real" programs become possible.

### Tier 2 — Power (make abstractions possible)

| Phase | Name | Estimated effort | Key unlock |
|---|---|---|---|
| 8 | Generics & Traits | 2 sprints | Reusable code, type-safe containers |
| 9 | Slices & Collections | 1 sprint | Vec, HashMap, dynamic data |
| 10 | I/O & Syscalls | 1 sprint | File/network/stdin/stdout |
| 11 | Closures & HOFs | 1 sprint | Functional patterns, callbacks |

**Milestone**: After Phase 11, Ny is a general-purpose language. Can build real CLI tools, data pipelines, file processors. Comparable to early Go (pre-generics).

### Tier 3 — Differentiator (make Ny worth choosing)

| Phase | Name | Estimated effort | Key unlock |
|---|---|---|---|
| 12 | Concurrency | 2 sprints | Multi-core compute, parallel ML |
| 13 | AI/ML Compute | 3 sprints | SIMD, tensors, GPU — the reason Ny exists |
| 14 | Tooling | Ongoing | LSP, formatter, packages — adoption |

**Milestone**: After Phase 13, Ny has a defensible niche. "The language where you write ML inference that runs as fast as hand-tuned C, in 1/5 the code."

---

## Competitive Positioning After Each Tier

### After Tier 1 (Phases 4–7)
**Ny vs Go**: Still inferior in every practical way. But the foundation is solid — memory model, error handling, and modules mean the language *could* grow.

### After Tier 2 (Phases 8–11)
**Ny vs Go**: Comparable for CLI tools and data processing. Faster execution (no GC), richer type system (generics + traits + pattern matching). Weaker ecosystem, no concurrency.
**Ny vs Zig**: Similar niche (low-level, no GC). Ny has friendlier syntax and ML focus. Zig has comptime and C interop.

### After Tier 3 (Phases 12–14)
**Ny vs Go**: Different leagues. Go is for web services. Ny is for compute.
**Ny vs Rust**: Simpler (no borrow checker), faster to write, narrower scope. Ny doesn't try to be Rust — it trades memory safety for ML ergonomics.
**Ny vs Mojo**: Direct competitor. Both target ML with Python-like ergonomics + native performance. Ny's edge: LLVM-native (not proprietary runtime), open toolchain, immutable-by-default.
**Ny vs C++/CUDA**: 10x less boilerplate for the same performance. One language for CPU + GPU. No header files, no build system hell.

---

## Risks & Mitigations

| Risk | Impact | Mitigation |
|---|---|---|
| Generics complexity explodes | Blocks all Tier 2+ | Start with monomorphization only, no trait objects. Keep it simple |
| GPU backend is LLVM-version-dependent | Phase 13 delayed | Abstract GPU behind trait interface, allow CPU fallback |
| No users until Phase 14 | Zero adoption | Ship formatter + LSP early (Phase 7+), build community with examples |
| Mojo/Zig ships the same features first | Positioning eroded | Focus on niche: immutable-by-default + ML-first. Don't try to be general-purpose |
| Single developer bottleneck | Slow progress | Open-source after Phase 7, attract contributors with clear spec-driven workflow |
| Memory safety concerns (no borrow checker) | Security criticism | Document the tradeoff explicitly. Add optional sanitizers (valgrind integration, bounds checking). Safety is a spectrum, not binary |

---

## Non-Goals (Explicitly Out of Scope)

- **Web backend framework** — Go and Rust own this. Don't compete.
- **Mobile/WASM target** — Focus on x86-64 + GPU. WASM can come later if demanded.
- **Dynamic typing** — Ny is statically typed, period.
- **Garbage collection** — Never. Manual + defer + future linear types.
- **OOP (classes, inheritance)** — Structs + traits. No class hierarchies.
- **Interpreted mode / REPL** — Compiled only. Debugging via LSP + debugger.
