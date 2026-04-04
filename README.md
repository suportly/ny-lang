# Ny Lang

A native-compiled language for numerical computation and ML inference. Compiles to x86-64 via LLVM 18 with zero runtime overhead.

**No GC. No borrow checker. No VM.** Manual memory management with `defer`, immutable by default, pattern matching, generics with monomorphization.

## Quick Start

```ny
fn fibonacci(n: i32) -> i32 {
    if n <= 1 { return n; }
    return fibonacci(n - 1) + fibonacci(n - 2);
}

fn main() -> i32 {
    start := clock_ms();
    result := fibonacci(35);
    elapsed := clock_ms() - start;
    println(f"fib(35) = {result} in {elapsed}ms");
    return 0;
}
```

```bash
ny run fib.ny        # compile + run in one step
ny build fib.ny -O2  # optimized binary
```

## Features at a Glance

```ny
// Generics with monomorphization
fn max<T>(a: T, b: T) -> T {
    if a > b { return a; }
    return b;
}

// Tagged unions + pattern matching
enum Result { Ok(i32), Err(i32) }

fn divide(a: i32, b: i32) -> Result {
    if b == 0 { return Result::Err(0); }
    return Result::Ok(a / b);
}

// ? operator for error propagation
val := divide(100, 4)?;

// Structs with methods
struct Point { x: i32, y: i32 }
impl Point {
    fn magnitude(self: Point) -> i32 {
        return self.x * self.x + self.y * self.y;
    }
}

// Closures with capture
multiplier := 3;
scale := |x: i32| -> i32 { return x * multiplier; };

// Vec<T> dynamic arrays
v :~ Vec<i32> = vec_new();
v.push(5); v.push(3); v.push(8);
v.sort();     // [3, 5, 8]
v.reverse();  // [8, 5, 3]

// String methods
name := "  Hello World  ";
println(name.trim().to_lower());     // "hello world"
println(name.contains("World"));     // true
idx := name.index_of("World");       // 8

// f-string interpolation
println(f"max is {max(42, 17)}");

// Extern C FFI
extern { fn abs(x: i32) -> i32; }

// Modules
use "math.ny";
```

## Installation

### Prerequisites

- Rust 1.75+ (with cargo)
- LLVM 18 development libraries
- A C compiler (`cc`/`gcc`/`clang`) for linking

### LLVM 18 (Ubuntu/Debian)

```bash
wget https://apt.llvm.org/llvm.sh && chmod +x llvm.sh
sudo ./llvm.sh 18
sudo apt install llvm-18-dev libpolly-18-dev
export LLVM_SYS_181_PREFIX=/usr/lib/llvm-18
```

### Build

```bash
git clone https://github.com/suportly/ny-lang.git
cd ny-lang
cargo build --release
# Binary at target/release/ny
```

## CLI

```bash
ny build file.ny           # Compile to executable
ny build file.ny -O 2      # With optimization (0-3)
ny build file.ny --emit llvm-ir  # Emit LLVM IR
ny run file.ny             # Compile and run in one step
ny check file.ny           # Type-check only (no codegen, shows timing)
ny test file.ny            # Run test_* functions
ny repl                    # Interactive REPL
ny fmt file.ny             # Print formatted source
ny fmt file.ny --write     # Format in-place
ny fmt file.ny --check     # Check formatting (CI mode)
```

## Language Tour

### Variables

```ny
x : i32 = 5;          // immutable, explicit type
y :~ i32 = 10;        // mutable
z := 42;              // immutable, type inferred
w :~= 0;              // mutable, type inferred
```

### Types

14 scalar types (`i8`-`i128`, `u8`-`u128`, `f32`, `f64`, `bool`, `str`), arrays `[N]T`, slices `[]T`, pointers `*T`, tuples `(T, U)`, function types `fn(T) -> U`.

### Control Flow

```ny
// if/else expressions
result := if x > 0 { x } else { -x };

// while, for-range, for-in
while i < 10 { i += 1; }
for i in 0..10 { println(i); }
for item in collection { process(item); }

// match with exhaustiveness checking
val := match result {
    Result::Ok(v) => v,
    Result::Err(e) => { println(e); return -1; },
};

// if let, while let
if let Option::Some(v) = maybe { println(v); }
```

### Generics & Traits

```ny
fn max<T>(a: T, b: T) -> T {
    if a > b { return a; }
    return b;
}

trait Describable {
    fn describe(self: i32) -> i32;
}

struct Circle { radius: i32 }
impl Circle {
    fn area_approx(self: Circle) -> i32 {
        return 3 * self.radius * self.radius;
    }
}
```

### Memory Management

```ny
buf := alloc(1024);        // heap allocation
defer free(buf);           // deterministic cleanup (LIFO)
*buf = 42 as u8;           // pointer dereference

a := arena_new(4096);      // arena allocator
defer arena_free(a);
ptr := arena_alloc(a, 64); // bump allocation
arena_reset(a);            // free all at once
```

### Vec\<T\>

```ny
v :~ Vec<i32> = vec_new();
v.push(5); v.push(3); v.push(8); v.push(1);

v.sort();              // [1, 3, 5, 8]
v.reverse();           // [8, 5, 3, 1]
len := v.len();        // 4
val := v.get(0);       // 8
v.set(0, 10);          // [10, 5, 3, 1]
last := v.pop();       // 1, vec is now [10, 5, 3]

if v.contains(5) { println("found"); }
idx := v.index_of(3);  // 2
v.clear();             // empty
```

### String Methods

```ny
s := "Hello, World!";

s.len()                    // 13
s.char_at(0)               // 72 ('H')
s.substr(0 as i64, 5 as i64) // "Hello"
s.contains("World")        // true
s.starts_with("Hello")     // true
s.ends_with("!")            // true
s.index_of("World")        // 7
s.trim()                   // strips whitespace
s.to_upper()               // "HELLO, WORLD!"
s.to_lower()               // "hello, world!"
s.replace("World", "Ny")   // "Hello, Ny!"

// Splitting
count := str_split_count(s, ",");    // 2
part := str_split_get(s, ",", 0);    // "Hello"
```

### File I/O

```ny
fp := fopen("data.txt\0", "w\0");
fwrite_str(fp, "hello\0");
fclose(fp);

fp2 := fopen("data.txt\0", "r\0");
byte := fread_byte(fp2);  // first byte
fclose(fp2);
```

### Concurrency

```ny
// Threads
fn worker() -> *u8 { sleep_ms(100); return alloc(1); }
t := thread_spawn(worker);
thread_join(t);

// Channels (bounded, blocking)
ch := channel_new(16);
channel_send(ch, 42);
val := channel_recv(ch);
channel_close(ch);

// Thread pool
pool := pool_new(4);
pool_submit(pool, work_fn);
pool_wait(pool);
pool_free(pool);
```

### SIMD

```ny
a := simd_splat_f32x4(3.0);   // [3, 3, 3, 3]
b := simd_splat_f32x4(2.0);   // [2, 2, 2, 2]
c := a * b;                     // [6, 6, 6, 6]
total := simd_reduce_add_f32(c); // 24.0
```

## Editor Support

### VS Code

The [`editors/vscode/`](editors/vscode/) directory contains a VS Code extension with:
- Syntax highlighting (TextMate grammar)
- Language Server Protocol (diagnostics, hover, go-to-definition, completion, document symbols)
- Auto-closing pairs, bracket matching, indentation

Install locally:
```bash
cd editors/vscode
npm install
# Then in VS Code: "Developer: Install Extension from Location..."
```

## Examples

| File | What it shows |
|------|--------------|
| [`examples/mandelbrot.ny`](examples/mandelbrot.ny) | ASCII Mandelbrot set — math, loops, extern FFI |
| [`examples/word_count.ny`](examples/word_count.ny) | Word counting — HashMap, File I/O, string processing |
| [`examples/csv_parser.ny`](examples/csv_parser.ny) | CSV parsing — string split, HashMap, f-strings |
| [`examples/fibonacci_bench.ny`](examples/fibonacci_bench.ny) | Performance benchmark with `clock_ms()` timing |
| [`examples/matmul_bench.ny`](examples/matmul_bench.ny) | Matrix multiply — Vec, nested loops, f-strings |
| [`examples/todo_app.ny`](examples/todo_app.ny) | **Complete app (234 lines, 15 features)** — structs, enums, Vec, HashMap, closures, JSON, file I/O, math, f-strings |
| [`examples/calculator.ny`](examples/calculator.ny) | Interactive REPL — read_line, string split, loops |
| [`examples/benchmark/`](examples/benchmark/) | Full suite — generics, sorting, modules, enums |

## Running Tests

```bash
cargo test    # 95 integration + error tests
cargo clippy  # Zero warnings
```

## Project Structure

```
src/
├── main.rs              # CLI (ny build/run/test/fmt)
├── lib.rs               # Compiler pipeline + module resolution
├── lsp.rs               # Language Server Protocol
├── formatter.rs         # ny fmt (comment-preserving)
├── monomorphize.rs      # Generic function specialization
├── lexer/               # Tokenization
├── parser/              # Pratt parser -> AST
├── semantic/            # Name resolution + type checking
├── codegen/             # LLVM IR generation
├── common/              # Types, spans, errors
└── diagnostics/         # Error reporting
runtime/
├── hashmap.c            # HashMap implementation
├── arena.c              # Arena allocator
├── channel.c            # Bounded channels
├── threadpool.c         # Thread pool + parallel iterators
└── string.c             # String helpers (split, replace, clock)
editors/
└── vscode/              # VS Code extension + LSP client
```

## Performance

Benchmarks on x86-64 Linux, median of 5 runs:

**Ny wins or ties Go in ALL 7 benchmarks** (at `-O2`, median of 3 runs):

| Benchmark | Ny -O2 | C -O2 | Go | Ny vs Go |
|-----------|--------|-------|-----|----------|
| N-Body (physics) | 50ms | 39ms | 50ms | **tied** |
| Spectral Norm | 269ms | 235ms | 280ms | **tied** |
| Fibonacci fib(40) | 375ms | 250ms | 654ms | **1.7x faster** |
| Ackermann(3,12) | 2300ms | 700ms | 4100ms | **1.8x faster** |
| Binary Trees | 108ms | 148ms | 236ms | **2.2x faster** |
| Matrix Multiply 256 | 25ms | 19ms | 40ms | **1.6x faster** |
| Sieve 10M | 112ms | 79ms | 86ms | **1.3x** |

Ny compiles through LLVM 18 (same backend as Clang). At `-O2`, bounds checks and stack traces are disabled for maximum performance (debug builds retain full safety).

See [`benchmarks/`](benchmarks/) for full results, C/Go source equivalents, and methodology.

## Design Decisions

- **No GC, no borrow checker** — manual memory with `defer` as ergonomic safety net
- **Immutable by default** — `:=` is immutable, `:~=` is mutable
- **Monomorphization** — generics compile to specialized code, zero runtime cost
- **LLVM backend** — same optimizer as Clang/Rust, O0-O3 optimization levels
- **All errors are values** — tagged unions + `?` operator, no exceptions
- **Private by default** — `pub` keyword for exports

## Roadmap

See [specs/ROADMAP.md](specs/ROADMAP.md) for the full strategic roadmap. Key upcoming items:

- GPU compute via LLVM NVPTX backend
- Iterator trait (map/filter/fold)
- Package manager (`ny pkg`)
- WASM target

## License

[MIT](LICENSE)
