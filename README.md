# Ny Lang

A low-level compiled language for AI/ML with extreme performance. Rust compiler with LLVM 18 backend, compiling to native x86-64 executables.

## Quick Example

```rust
use "math.ny";

enum Result { Ok(i32), Err(i32) }

fn safe_div(a: i32, b: i32) -> Result {
    if b == 0 { return Result::Err(0); }
    return Result::Ok(a / b);
}

fn main() -> i32 {
    // Generics
    bigger := max(42, 17);

    // Tagged unions + ? operator
    val := safe_div(100, 4)?;

    // Vec + for-in
    v :~ Vec<i32> = vec_new();
    for i in 0..10 { v.push(i * i); }
    println(v);  // [0, 1, 4, 9, 16, 25, 36, 49, 64, 81]

    // Closures with capture
    multiplier := 3;
    scale := |x: i32| -> i32 { return x * multiplier; };
    println(scale(val));

    return 0;
}
```

```bash
ny build main.ny -O 2 -o app && ./app
```

## Features

### Type System
- **14 scalar types** — i8/i16/i32/i64/i128, u8/u16/u32/u64/u128, f32/f64, bool, str
- **Arrays** — `[N]T` fixed-size with bounds checking
- **Structs** — named fields, impl blocks, methods with `self`
- **Enums** — tagged unions with data payloads: `enum Result { Ok(i32), Err(i32) }`
- **Tuples** — `(i32, bool)` with destructuring
- **Slices** — `[]T` borrowed views with `.len()` and indexing
- **Vec** — `Vec<i32>`, `Vec<f64>` dynamic arrays with push/get/len
- **HashMap** — `map_new()` / `map_insert()` / `map_get()` string-to-int mapping
- **Pointers** — `*T`, address-of `&`, dereference `*`, pointer arithmetic `ptr + n`
- **Function pointers** — `fn(i32) -> i32` as first-class values

### Control Flow
- **if/else** expressions, **while** loops, **for** ranges (`0..10`, `0..=10`)
- **for-in** iteration over arrays, slices, and Vec
- **loop** infinite loops with break/continue
- **match** expressions with exhaustiveness checking
- **if let** pattern matching: `if let Ok(v) = result { ... }`
- **defer** for deterministic cleanup (LIFO order)

### Functions & Abstractions
- **Generic functions** — `fn max<T>(a: T, b: T) -> T` with monomorphization
- **Impl blocks** — `impl Point { fn distance(self: Point) -> f64 { ... } }`
- **Traits** — `trait Describable { fn describe(self: T) -> i32; }` with conformance checking
- **Lambdas** — `|x: i32| -> i32 { x * 2 }` non-capturing function pointers
- **Closures** — `|x: i32| -> i32 { x * n }` capturing variables from outer scope
- **Void functions** — `fn greet(name: str) { println(name); }` without `-> ()` annotation
- **? operator** — `val := divide(10, 0)?;` error propagation on tagged unions

### Memory Management
- **Immutable by default** — `:` immutable, `:~` mutable, `::` compile-time constant
- **Heap allocation** — `alloc(size)`, `free(ptr)`, `sizeof(expr)`
- **defer** — automatic cleanup on all exit paths
- **No GC** — manual memory management with zero runtime overhead

### Modules & Interop
- **Module imports** — `use "math.ny";` multi-file programs
- **Extern C FFI** — `extern { fn abs(x: i32) -> i32; }` call any C library
- **C runtime linking** — HashMap backed by C implementation auto-linked

### Tooling
- **ny build** — compile to native executable with `-O0` to `-O3`
- **ny test** — run `test_*` functions with pass/fail reporting
- **LLVM IR output** — `--emit llvm-ir` for inspection
- **Error diagnostics** — source locations with code snippets

## Builtin Functions

| Function | Signature | Description |
|----------|-----------|-------------|
| `print` | `(any) -> ()` | Print to stdout |
| `println` | `(any) -> ()` | Print with newline |
| `alloc` | `(i32) -> *u8` | Heap allocation |
| `free` | `(*u8) -> ()` | Free heap memory |
| `sizeof` | `(any) -> i64` | Size of type in bytes |
| `vec_new` | `() -> Vec<T>` | Create empty vector |
| `map_new` | `() -> *u8` | Create empty hashmap |
| `map_insert` | `(*u8, str, i32) -> ()` | Insert key-value |
| `map_get` | `(*u8, str) -> i32` | Get value by key |
| `map_contains` | `(*u8, str) -> bool` | Check key exists |
| `map_len` | `(*u8) -> i64` | Number of entries |
| `fopen` | `(str, str) -> *u8` | Open file |
| `fclose` | `(*u8) -> i32` | Close file |
| `fwrite_str` | `(*u8, str) -> i32` | Write string to file |
| `fread_byte` | `(*u8) -> i32` | Read byte from file |
| `read_line` | `() -> str` | Read line from stdin |
| `int_to_str` | `(i32) -> str` | Integer to string |
| `str_to_int` | `(str) -> i32` | String to integer |
| `exit` | `(i32) -> !` | Exit process |
| `sleep_ms` | `(i32) -> ()` | Sleep milliseconds |

## Language Syntax

```rust
// Variables
x : i32 = 5;          // immutable with type
y :~ i32 = 10;        // mutable
z := 42;              // type inference (immutable)
w :~= 0;              // type inference (mutable)
PI :: f64 = 3.14;     // compile-time constant

// Functions
fn square(x: i32) -> i32 = x * x;   // expression body
fn greet(name: str) { println(name); }  // void function

// Generics
fn max<T>(a: T, b: T) -> T { if a > b { return a; } return b; }

// Structs + impl
struct Point { x: i32, y: i32 }
impl Point {
    fn distance(self: Point) -> i32 { return self.x + self.y; }
}

// Enums (tagged unions)
enum Option { Some(i32), None }
result := Option::Some(42);
if let Option::Some(v) = result { println(v); }

// Closures
n := 5;
add_n := |x: i32| -> i32 { return x + n; };

// Extern FFI
extern { fn rand() -> i32; fn abs(x: i32) -> i32; }
```

## Performance

Fibonacci(40) benchmark on x86-64 Linux with `-O2`:

| Language | Avg Time | vs C |
|----------|----------|------|
| C (gcc -O2) | 0.25s | 1.0x |
| **Ny (-O2)** | **0.48s** | **1.9x** |
| Go | 0.72s | 2.9x |

Ny compiles to native LLVM IR — **33% faster than Go** on compute-heavy workloads.

## Installation

### Prerequisites

- Rust 1.75+ (with cargo)
- LLVM 18.x development libraries
- A C compiler (`cc` / `gcc` / `clang`) for linking

### LLVM 18 (Ubuntu/Debian)

```bash
wget https://apt.llvm.org/llvm.sh
chmod +x llvm.sh
sudo ./llvm.sh 18
sudo apt install llvm-18-dev libpolly-18-dev
export LLVM_SYS_181_PREFIX=/usr/lib/llvm-18
```

### Build from source

```bash
git clone https://github.com/suportly/ny-lang.git
cd ny-lang
cargo build --release
```

## Usage

```bash
ny build program.ny              # Compile to executable
ny build program.ny -o app       # Custom output path
ny build program.ny -O 2         # Optimized build
ny build program.ny --emit llvm-ir  # Emit LLVM IR
ny test program.ny               # Run test_* functions
```

## Examples

- [`examples/benchmark/`](examples/benchmark/) — full benchmark suite (sorting, fibonacci, generics, Vec, modules)
- [`examples/fibonacci.ny`](examples/fibonacci.ny) — recursive Fibonacci
- [`examples/hello.ny`](examples/hello.ny) — simplest program

## Running Tests

```bash
cargo test    # 58 tests (47 integration + 11 error)
cargo clippy  # Lint
```

## Project Structure

```
src/
├── main.rs           # CLI (ny build, ny test)
├── lib.rs            # Compiler pipeline + module resolution
├── monomorphize.rs   # Generic function specialization
├── lexer/            # Tokenization
├── parser/           # Pratt parser → AST
├── semantic/         # Name resolution + type checking
├── codegen/          # LLVM IR generation
├── diagnostics/      # Error reporting
└── common/           # Types, spans, errors
runtime/
└── hashmap.c         # C runtime for HashMap
specs/                # Feature specifications (001–014)
tests/                # 58 tests
```

## License

[MIT](LICENSE)
