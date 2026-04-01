# Quickstart: LNGE Compiler

## Prerequisites

- Rust 1.75+ (with cargo)
- LLVM 18.x development libraries
- A C compiler (`cc` / `gcc` / `clang`) for linking

### Installing LLVM 18 (Ubuntu/Debian)

```bash
wget https://apt.llvm.org/llvm.sh
chmod +x llvm.sh
sudo ./llvm.sh 18
sudo apt install llvm-18-dev libpolly-18-dev
```

Set the LLVM prefix for inkwell:

```bash
export LLVM_SYS_180_PREFIX=/usr/lib/llvm-18
```

## Build

```bash
cargo build
```

## Run the compiler

### Compile a program

Create `hello.lnge`:

```lnge
fn main() -> i32 {
    return 42;
}
```

Compile and run:

```bash
cargo run -- build hello.lnge
./hello
echo $?  # prints 42
```

### Fibonacci example

Create `fib.lnge`:

```lnge
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
cargo run -- build fib.lnge -o fib
./fib
echo $?  # prints 55
```

## Run tests

```bash
cargo test
```

## Project structure

```
src/
├── main.rs          # CLI entry point
├── lib.rs           # Library root
├── lexer/           # Tokenization
├── parser/          # Pratt parsing → AST
├── semantic/        # Name resolution + type checking
├── codegen/         # LLVM IR generation
├── diagnostics/     # Error reporting
└── common/          # Shared types (spans, types)

tests/
├── integration/     # End-to-end compile+run tests
├── unit/            # Per-module tests
└── fixtures/        # .lnge test programs
```
