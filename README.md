# Ny Lang

A low-level compiled language for AI/ML with extreme performance. Rust compiler with LLVM 18 backend.

## Quick Example

```rust
fn fibonacci(n: i32) -> i32 {
    if n <= 1 {
        return n;
    }
    return fibonacci(n - 1) + fibonacci(n - 2);
}

fn main() -> i32 {
    return fibonacci(10);
}
```

```bash
ny build fibonacci.ny -O 2 -o fib
./fib
echo $?  # 55
```

## Features

- **Scalar types** — i8, i16, i32, i64, i128, u8, u16, u32, u64, u128, f32, f64, bool
- **Functions** — typed parameters, return types, recursive calls, expression-body syntax
- **Immutable by default** — `:` for immutable, `:~` for mutable, `::` for compile-time constants
- **Control flow** — if/else expressions, while loops, block expressions
- **Pratt parser** — correct operator precedence for arithmetic, comparison, and logical operators
- **LLVM backend** — optimization levels O0–O3 with host CPU feature detection
- **Clear errors** — source locations, code snippets, and descriptive messages via codespan-reporting

## Language Syntax

```rust
// Immutable variable
x : i32 = 5;

// Mutable variable
y :~ i32 = 10;
y = y + 1;

// Compile-time constant
PI :: f64 = 3.14;

// Expression-body function
fn square(x: i32) -> i32 = x * x;

// Block-body function
fn abs(x: i32) -> i32 {
    if x < 0 { return -x; }
    return x;
}
```

## Performance

Fibonacci(40) benchmark on x86-64 Linux with `-O2`:

| Language | Avg Time | vs C |
|----------|----------|------|
| C (gcc -O2) | 0.25s | 1.0x |
| **Ny (-O2)** | **0.48s** | **1.9x** |
| Go | 0.72s | 2.9x |

Ny is **33% faster than Go** on compute-heavy workloads. The gap to C can be narrowed with future optimizations (tail-call, iteration lowering).

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

The binary is at `target/release/ny`. Optionally install it:

```bash
cargo install --path .
```

## Usage

```bash
# Compile to executable
ny build program.ny

# Specify output path
ny build program.ny -o myapp

# Compile with optimization
ny build program.ny -O 2

# Emit LLVM IR
ny build program.ny --emit llvm-ir

# Emit object file
ny build program.ny --emit obj
```

## Examples

See the [`examples/`](examples/) directory:

- [`hello.ny`](examples/hello.ny) — simplest program (returns 42)
- [`fibonacci.ny`](examples/fibonacci.ny) — recursive Fibonacci with functions and control flow
- [`variables.ny`](examples/variables.ny) — immutable/mutable variables, constants, expression-body functions

## Running Tests

```bash
cargo test
```

12 integration tests: 7 compile-and-run + 5 error detection.

## Project Structure

```
src/
├── main.rs          # CLI entry point (clap)
├── lib.rs           # Library root
├── lexer/           # Tokenization
├── parser/          # Pratt parsing → AST
├── semantic/        # Name resolution + type checking
├── codegen/         # LLVM IR generation
├── diagnostics/     # Error reporting
└── common/          # Shared types (spans, types)
```

## License

[MIT](LICENSE)
