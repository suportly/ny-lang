# Syntax Contract: Ny Lang Phase 2 Extensions

**Date**: 2026-04-01

## Array Syntax

```ny
// Array type annotation
a : [5]i32 = [1, 2, 3, 4, 5];

// Mutable array
b :~ [3]f64 = [1.0, 2.0, 3.0];
b[0] = 9.0;

// Nested arrays (2D matrix)
m : [3][3]i32 = [[1,2,3], [4,5,6], [7,8,9]];

// Array as function parameter
fn sum(arr: [5]i32) -> i32 { ... }

// Array pointer parameter
fn zero(arr: *[5]i32) { ... }
```

## Struct Syntax

```ny
// Struct definition (top-level only)
struct Vec2 {
    x: f64,
    y: f64,
}

// Struct instantiation
v : Vec2 = Vec2 { x: 3.0, y: 4.0 };

// Field access
println(v.x);

// Method definition (self by value)
fn length(self: Vec2) -> f64 {
    return sqrt(self.x * self.x + self.y * self.y);
}

// Method definition (self by pointer, for mutation)
fn scale(self: *Vec2, factor: f64) {
    self.x = self.x * factor;  // auto-deref
    self.y = self.y * factor;
}

// Method call
v.length();
v.scale(2.0);  // requires &v or mutable v
```

## Pointer Syntax

```ny
// Address-of
x :~ i32 = 42;
p : *i32 = &x;

// Dereference read
y : i32 = *p;

// Dereference write
*p = 99;

// Struct pointer auto-deref
sp : *Vec2 = &v;
sp.x;         // equivalent to (*sp).x

// Array pointer
ap : *[5]i32 = &arr;
```

## String Syntax

```ny
// String literal
msg : str = "Hello, Ny!";

// Escape sequences
newline : str = "line1\nline2";
tab : str = "col1\tcol2";
quote : str = "say \"hello\"";
backslash : str = "path\\file";

// Print
print("no newline");
println("with newline");
println(42);
println(3.14);
println(true);
println(v);  // Vec2 { x: 3.0, y: 4.0 }
```

## For Loop Syntax

```ny
// Exclusive range (0, 1, 2, ..., 9)
for i in 0..10 {
    println(i);
}

// Inclusive range (0, 1, 2, ..., 10)
for i in 0..=10 {
    println(i);
}

// With break
for i in 0..100 {
    if arr[i] == target {
        break;
    }
}

// With continue
for i in 0..10 {
    if i % 2 != 0 {
        continue;
    }
    println(i);  // only even numbers
}
```

## Type Inference Syntax

```ny
// Immutable inferred (default types: i32, f64, bool)
x := 5;          // i32
y := 3.14;        // f64
b := true;        // bool
v := Vec2 { x: 1.0, y: 2.0 };  // Vec2

// Mutable inferred
count :~= 0;     // i32, mutable
count = count + 1;
```

## Break and Continue

```ny
// Break exits innermost loop
while true {
    if done {
        break;
    }
}

// Continue skips to next iteration
for i in 0..10 {
    if skip_condition {
        continue;
    }
    // process i
}

// Nested loops — break/continue affect innermost only
for i in 0..10 {
    for j in 0..10 {
        if j == 5 {
            break;  // exits inner loop only
        }
    }
    // i loop continues
}
```
