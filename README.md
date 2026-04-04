# Ny Lang

**Go's concurrency + Rust's type safety.** Native compiled via LLVM 18.

Goroutines, typed channels, select, pattern matching, traits with dynamic dispatch — compiled to native x86-64 and WebAssembly. Faster than Go on compute-bound workloads.

```ny
fn main() {
    ch : chan<i32> = chan_new(10);

    go producer(ch, 21);
    go producer(ch, 21);

    var total = ch.recv() + ch.recv();
    println("total:", total);  // total: 42
}

fn producer(ch : chan<i32>, value: i32) {
    ch.send(value);
}
```

```bash
ny run main.ny        # compile + run
ny build main.ny -O2  # optimized binary
```

## Why Ny?

- **Concurrency built-in**: `go`, `chan<T>`, `select` — no frameworks, no async coloring
- **Type safety without complexity**: enums, pattern matching, `?T` optionals, `dyn Trait` — no borrow checker
- **Fast**: 1.3-2.2x faster than Go, within 1.5x of C on all benchmarks

## Language Tour

### Variables

```ny
result := 42;           // immutable, type inferred
var total = 0;          // mutable (Go-style)
name : str = "hello";   // explicit type
```

### Structs + Methods

```ny
struct Point { x: i32, y: i32 }

impl Point {
    fn magnitude(self: Point) -> i32 {
        return self.x * self.x + self.y * self.y;
    }
}

p := new Point { x: 3, y: 4 };  // GC-managed heap allocation
println(p.magnitude());          // 25
```

### Enums + Pattern Matching

```ny
enum Result {
    Ok(i32),
    Err(str),    // string error messages
}

fn divide(a: i32, b: i32) -> Result {
    if b == 0 { return Result::Err("division by zero"); }
    return Result::Ok(a / b);
}

match divide(10, 3) {
    Result::Ok(v) => println("result:", v),
    Result::Err(msg) => println("error:", msg),
}
```

### Traits + Dynamic Dispatch

```ny
interface Shape {
    fn area(self: i32) -> i32;
}

struct Circle { radius: i32 }
impl Shape for Circle {
    fn area(self: Circle) -> i32 { return self.radius * self.radius * 3; }
}

// dyn Shape = interface value (fat pointer with vtable)
fn print_area(s: dyn Shape) { println("area:", s.area()); }
```

### Error Handling

```ny
fn compute() -> Result {
    x := divide(84, 2)?;   // unwraps Ok or propagates Err
    y := divide(x, 0)?;    // error: "division by zero"
    return Result::Ok(y);
}

// Rich errors with messages + stack traces
code := error_new("something went wrong");
msg := error_message(code);    // "something went wrong"
trace := error_trace(code);    // call stack at error creation
```

### Goroutines + Channels + Select

```ny
ch1 : chan<i32> = chan_new(16);
ch2 : chan<i32> = chan_new(16);

go producer(ch1, 20);
go producer(ch2, 22);

// select: receive from first ready channel
select {
    v := ch1.recv() => { println("ch1:", v); },
    v := ch2.recv() => { println("ch2:", v); },
}
```

### Optional Types + Null Safety

```ny
p : ?*Point = nil;

// Compile error: cannot access field on optional type
// p.x;

// Safe unwrap with if let
if let val = p {
    println(val.x);      // only runs if non-nil
}

// Null coalescing
safe := p ?? new Point { x: 0, y: 0 };
println(safe.x);         // always safe
```

### Generics

```ny
fn max<T>(a: T, b: T) -> T {
    if a > b { return a; }
    return b;
}

println(max(42, 17));      // 42
println(max(3.14, 2.71));  // 3.14
```

### Collections

```ny
// Vec<T> with 20 methods
var v : Vec<i32> = vec_new();
v.push(5); v.push(3); v.push(8);
v.sort();
total := v.reduce(|a: i32, b: i32| -> i32 { return a + b; }, 0);

// HashMap with Go-style iteration
m := map_new();
map_insert(m, "hello", 42);
for key, value in m {
    println(key, value);
}

// Type aliases
type Meters = f64;
type UserID = i32;
```

## Performance

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

Compiled through LLVM 18 (same backend as Clang). At `-O2`, bounds checks and stack traces are disabled for maximum performance.

## Tooling

| Tool | Command | Description |
|------|---------|-------------|
| Build | `ny build file.ny` | Compile to native binary |
| Run | `ny run file.ny` | Compile and run |
| Check | `ny check file.ny` | Type-check without compiling |
| Test | `ny test file.ny` | Run `test_*` functions |
| Format | `ny fmt file.ny --write` | Auto-format source |
| REPL | `ny repl` | Interactive mode |
| LSP | `ny-lsp` | Language server (6 capabilities) |
| Package | `ny pkg add <url>` | Git-based dependency management |
| WASM | `ny build file.ny --target wasm32` | Compile to WebAssembly |

**VS Code extension**: syntax highlighting + LSP client in [`editors/vscode/`](editors/vscode/).

## Known Limitations

- **Concurrency**: `go` uses a fixed-size OS thread pool, not green threads. Efficient for CPU-bound workloads with few goroutines; not designed for 100k+ lightweight tasks.
- **GC**: Mark-and-sweep, stop-the-world. No generational collection. Pausas proportional to heap size.
- **Error handling**: `error_new`/`error_message` use a global table, not typed error objects. Stack traces only in debug builds.
- **Generics**: Monomorphization (like C++/Rust). Code size grows with type instantiations. Use `dyn Trait` for code-size-sensitive paths.
- **`async`/`await`**: Deprecated. Use `go` + channels instead.

## Building from Source

```bash
# Requirements: Rust 1.75+, LLVM 18, clang
git clone https://github.com/suportly/ny-lang.git
cd ny-lang
cargo build --release
```

## Project Structure

```text
src/            Compiler (Rust + inkwell/LLVM 18)
runtime/        C runtime (12 files: GC, channels, threadpool, tensors, ...)
editors/vscode/ VS Code extension + syntax highlighting
tests/          146 tests (integration + negative)
benchmarks/     7 benchmarks with C + Go equivalents
examples/       Example programs
specs/          Roadmap and feature specifications
```

## License

[MIT](LICENSE)
