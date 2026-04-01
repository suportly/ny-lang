# CLI Contract: lnge

**Date**: 2026-04-01
**Binary name**: `lnge`

## Commands

### `lnge build <FILE>`

Compile an LNGE source file to a native executable.

**Arguments**:

| Argument | Required | Description                    |
|----------|----------|--------------------------------|
| `FILE`   | Yes      | Path to `.lnge` source file    |

**Options**:

| Option             | Short | Default        | Description                            |
|--------------------|-------|----------------|----------------------------------------|
| `--output <PATH>`  | `-o`  | `<FILE stem>`  | Output executable path                 |
| `--emit <TYPE>`    |       | `exe`          | Output type: `exe`, `llvm-ir`, `obj`   |
| `--opt-level <N>`  | `-O`  | `0`            | Optimization level: 0, 1, 2, 3         |

**Exit codes**:

| Code | Meaning                                    |
|------|--------------------------------------------|
| 0    | Compilation successful                     |
| 1    | Compilation failed (source errors)         |
| 2    | I/O error (file not found, write failure)  |

## Input Format

LNGE source files (`.lnge` extension by convention, not enforced).

### Minimal valid program

```lnge
fn main() -> i32 {
    return 0;
}
```

### Variable declarations

```lnge
x : i32 = 5;       // immutable
y :~ i32 = 10;     // mutable
PI :: f64 = 3.14;  // compile-time constant
```

### Function definitions

```lnge
fn add(a: i32, b: i32) -> i32 {
    return a + b;
}

// Expression body shorthand
fn square(x: i32) -> i32 = x * x;
```

### Control flow

```lnge
if condition {
    // ...
} else {
    // ...
}

while x > 0 {
    x = x - 1;
}
```

## Output Format

### Success (exit code 0)

No stdout output. The compiled executable is written to the output path.

```
$ lnge build hello.lnge
$ ./hello
$ echo $?
0
```

### Compilation error (exit code 1)

Errors are written to stderr with source locations and code snippets:

```
error[E001]: type mismatch
  --> hello.lnge:3:12
   |
 3 |     x : i32 = true;
   |               ^^^^ expected `i32`, found `bool`
```

### I/O error (exit code 2)

```
error: could not read file `missing.lnge`: No such file or directory
```

## Supported Types

| Type | Description              | Size    |
|------|--------------------------|---------|
| i8   | Signed 8-bit integer     | 1 byte  |
| i16  | Signed 16-bit integer    | 2 bytes |
| i32  | Signed 32-bit integer    | 4 bytes |
| i64  | Signed 64-bit integer    | 8 bytes |
| i128 | Signed 128-bit integer   | 16 bytes|
| u8   | Unsigned 8-bit integer   | 1 byte  |
| u16  | Unsigned 16-bit integer  | 2 bytes |
| u32  | Unsigned 32-bit integer  | 4 bytes |
| u64  | Unsigned 64-bit integer  | 8 bytes |
| u128 | Unsigned 128-bit integer | 16 bytes|
| f32  | 32-bit floating point    | 4 bytes |
| f64  | 64-bit floating point    | 8 bytes |
| bool | Boolean (true/false)     | 1 byte  |

## Operators

| Category    | Operators              | Precedence (high→low) |
|-------------|------------------------|-----------------------|
| Unary       | `-` (negate), `!` (not)| Highest               |
| Multiplicative | `*`, `/`, `%`       | 9-10                  |
| Additive    | `+`, `-`               | 7-8                   |
| Comparison  | `==`, `!=`, `<`, `>`, `<=`, `>=` | 5-6       |
| Logical AND | `&&`                   | 3-4                   |
| Logical OR  | `\|\|`                 | 1-2                   |
