# Quickstart: Ny Lang Phase 2

## Prerequisites

Same as Phase 1:
- Rust 1.75+ (with cargo)
- LLVM 18.x development libraries
- A C compiler (`cc` / `gcc` / `clang`) for linking

## Build

```bash
cargo build --release
```

## Examples

### Arrays and For Loops

Create `dot_product.ny`:

```ny
fn dot(a: [5]f64, b: [5]f64) -> f64 {
    sum :~= 0.0;
    for i in 0..5 {
        sum = sum + a[i] * b[i];
    }
    return sum;
}

fn main() -> i32 {
    a : [5]f64 = [1.0, 2.0, 3.0, 4.0, 5.0];
    b : [5]f64 = [5.0, 4.0, 3.0, 2.0, 1.0];
    result := dot(a, b);
    println(result);
    return 0;
}
```

```bash
ny build dot_product.ny -O 2
./dot_product
# Output: 35.000000
```

### Structs and Methods

Create `vec2.ny`:

```ny
struct Vec2 {
    x: f64,
    y: f64,
}

fn add(self: Vec2, other: Vec2) -> Vec2 {
    return Vec2 { x: self.x + other.x, y: self.y + other.y };
}

fn dot(self: Vec2, other: Vec2) -> f64 {
    return self.x * other.x + self.y * other.y;
}

fn main() -> i32 {
    a := Vec2 { x: 3.0, y: 4.0 };
    b := Vec2 { x: 1.0, y: 2.0 };
    c := a.add(b);
    println(c);
    println(a.dot(b));
    return 0;
}
```

```bash
ny build vec2.ny
./vec2
# Output:
# Vec2 { x: 4.000000, y: 6.000000 }
# 11.000000
```

### Pointers and Mutation

Create `pointers.ny`:

```ny
fn swap(a: *i32, b: *i32) {
    tmp := *a;
    *a = *b;
    *b = tmp;
}

fn main() -> i32 {
    x :~= 10;
    y :~= 20;
    println(x);
    println(y);
    swap(&x, &y);
    println(x);
    println(y);
    return 0;
}
```

```bash
ny build pointers.ny
./pointers
# Output:
# 10
# 20
# 20
# 10
```

### Hello World with Strings

Create `hello.ny`:

```ny
fn main() -> i32 {
    println("Hello, Ny!");
    println(42);
    println(3.14);
    println(true);
    return 0;
}
```

```bash
ny build hello.ny
./hello
# Output:
# Hello, Ny!
# 42
# 3.140000
# true
```

## Run Tests

```bash
cargo test
```
