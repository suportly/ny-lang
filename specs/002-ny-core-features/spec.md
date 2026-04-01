# Feature Specification: Ny Lang Core Features (Phase 2)

**Feature Branch**: `002-ny-core-features`
**Created**: 2026-04-01
**Status**: Draft
**Input**: User description: "Analyze Go lang and what Ny Lang needs in terms of features — arrays, strings, structs, pointers, for loops, print, type inference"

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Work with Arrays and Indexed Data (Priority: P1)

A developer writes a Ny program that declares fixed-size arrays of scalars, accesses elements by index, iterates over them with a `for` loop, and performs computations like dot products and sum reductions. The compiler correctly type-checks array operations and generates efficient code for element access.

**Why this priority**: Arrays are the foundational data structure for AI/ML. Without arrays, no real computation beyond scalar arithmetic is possible. Every other feature (structs, slices, tensors) builds on arrays.

**Independent Test**: Write a program that creates an array of 5 integers, sums them in a `for` loop, and returns the sum as the exit code. Compile with `ny build` and verify the result.

**Acceptance Scenarios**:

1. **Given** a program declaring `a : [5]i32 = [1, 2, 3, 4, 5];`, **When** compiled, **Then** the array is stack-allocated and accessible by index.
2. **Given** a program accessing `a[i]` where `i` is within bounds, **When** compiled and run, **Then** the correct element value is returned.
3. **Given** a program accessing `a[i]` where `i` is out of bounds at runtime, **When** run, **Then** the program terminates with an error (bounds check panic).
4. **Given** a program with `for i in 0..5 { sum = sum + a[i]; }`, **When** compiled, **Then** the loop iterates over the range and produces the correct sum.
5. **Given** an array passed to a function by value, **When** the function modifies an element, **Then** the original array is not affected (value semantics).

---

### User Story 2 - Define and Use Structs (Priority: P1)

A developer defines custom data types using structs with named fields, creates instances, accesses fields, and defines methods on structs. Structs enable organizing related data — e.g., representing a 2D vector with `x` and `y` fields.

**Why this priority**: Structs are essential for representing any multi-field data (vectors, matrices, neural network layers, configuration). Without structs, every multi-value computation requires passing individual scalars.

**Independent Test**: Define a `Vec2` struct with `x: f64, y: f64` fields, create an instance, compute the dot product via a method, and return the integer part of the result.

**Acceptance Scenarios**:

1. **Given** a struct definition `struct Vec2 { x: f64, y: f64 }`, **When** compiled, **Then** the struct type is recognized and usable.
2. **Given** a struct instantiation `v : Vec2 = Vec2 { x: 3.0, y: 4.0 };`, **When** compiled, **Then** the fields are correctly initialized.
3. **Given** field access `v.x`, **When** compiled, **Then** the correct field value is returned.
4. **Given** a method `fn length(self: Vec2) -> f64`, **When** called on an instance as `v.length()`, **Then** it computes and returns the correct result.
5. **Given** a struct passed to a function by value, **When** the function modifies a field, **Then** the original struct is not affected (value semantics).

---

### User Story 3 - Use Pointers for Efficient Data Passing (Priority: P1)

A developer uses pointers to pass large structs and arrays to functions without copying, and to mutate data through function calls. Pointers enable efficient manipulation of data in-place.

**Why this priority**: Without pointers, every function call copies data. For AI/ML workloads with large arrays and matrices, this makes the language unusable for real work.

**Independent Test**: Write a program that passes an array pointer to a function, modifies it through the pointer, and verifies the modification is visible to the caller.

**Acceptance Scenarios**:

1. **Given** a pointer type `*i32`, **When** a variable is declared as `p : *i32 = &x;`, **Then** `p` points to `x`.
2. **Given** a dereference operation `*p`, **When** `p` is a valid pointer, **Then** the pointed-to value is read correctly.
3. **Given** `*p = 42;`, **When** `p` points to a mutable variable, **Then** the pointed-to variable is updated to 42.
4. **Given** a function `fn zero_out(arr: *[5]i32)`, **When** called with `&my_array`, **Then** the function can modify the original array through the pointer.
5. **Given** a struct pointer `sp : *Vec2 = &v;`, **When** accessing `sp.x` (auto-deref), **Then** the field is accessible without explicit `(*sp).x`.

---

### User Story 4 - Print Output and Use Strings (Priority: P1)

A developer writes programs that produce visible output to stdout using `print` and `println` built-in functions, with support for string literals and basic string formatting. This makes programs observable and debuggable.

**Why this priority**: Currently Ny programs can only communicate results through exit codes (0-255). Without print, programs cannot output results larger than 255, display text, or log debugging information.

**Independent Test**: Write a program that prints "Hello, Ny!" followed by a computed number. Compile, run, and verify stdout contains the expected output.

**Acceptance Scenarios**:

1. **Given** `println("Hello, Ny!");`, **When** compiled and run, **Then** stdout contains `Hello, Ny!` followed by a newline.
2. **Given** `print(42);`, **When** compiled and run, **Then** stdout contains `42` with no trailing newline.
3. **Given** `println(fibonacci(10));`, **When** compiled and run, **Then** stdout contains `55` followed by a newline.
4. **Given** a string variable `msg : str = "result: ";`, **When** used with `print(msg);`, **Then** the string is printed to stdout.
5. **Given** `println(true);` and `println(3.14);`, **When** compiled and run, **Then** the boolean and float are printed in human-readable form.

---

### User Story 5 - Use For Loops with Ranges (Priority: P2)

A developer writes `for` loops that iterate over numeric ranges, replacing verbose `while` loop patterns with concise iteration syntax. Both `for i in 0..n` (exclusive) and `for i in 0..=n` (inclusive) range forms are supported.

**Why this priority**: The current `while` loop with manual index management is error-prone and verbose. `for` loops are standard in every modern language and essential for readable array iteration code.

**Independent Test**: Write a program that sums integers from 0 to 9 using `for i in 0..10` and returns the sum (45).

**Acceptance Scenarios**:

1. **Given** `for i in 0..10 { sum = sum + i; }`, **When** compiled and run, **Then** `i` takes values 0 through 9 and sum equals 45.
2. **Given** `for i in 0..=10 { sum = sum + i; }`, **When** compiled and run, **Then** `i` takes values 0 through 10 and sum equals 55.
3. **Given** `for i in 0..0 { ... }`, **When** compiled and run, **Then** the loop body does not execute (empty range).
4. **Given** a `for` loop with `break`, **When** the break condition is met, **Then** the loop exits immediately.
5. **Given** a `for` loop with `continue`, **When** the continue condition is met, **Then** the current iteration is skipped.

---

### User Story 6 - Local Type Inference (Priority: P2)

A developer declares variables without explicit type annotations, and the compiler infers the type from the initialization expression. This reduces verbosity while maintaining full type safety.

**Why this priority**: Writing `x : i32 = 5;` on every variable is verbose. Type inference lets developers write `x := 5;` when the type is obvious from context, matching the ergonomics of Go and Rust.

**Independent Test**: Write a program using `:=` inference for integer, float, bool, and struct variables. Verify it compiles and produces correct results.

**Acceptance Scenarios**:

1. **Given** `x := 5;`, **When** compiled, **Then** `x` is inferred as `i32` (default integer type) and is immutable.
2. **Given** `y := 3.14;`, **When** compiled, **Then** `y` is inferred as `f64` (default float type).
3. **Given** `b := true;`, **When** compiled, **Then** `b` is inferred as `bool`.
4. **Given** `v := Vec2 { x: 1.0, y: 2.0 };`, **When** compiled, **Then** `v` is inferred as `Vec2`.
5. **Given** `x := 5;` followed by `x = 10;`, **When** compiled, **Then** the compiler reports an immutability error (`:=` creates immutable variables).
6. **Given** `x :~= 5;`, **When** compiled, **Then** `x` is inferred as `i32` and is mutable (can be reassigned).

---

### User Story 7 - Break and Continue in Loops (Priority: P2)

A developer uses `break` to exit loops early and `continue` to skip iterations, enabling common patterns like search-and-exit and filter-in-loop.

**Why this priority**: These are fundamental control flow statements missing from Phase 1. Without them, early exit from loops requires flag variables and extra conditions.

**Independent Test**: Write a program that searches an array for a value and returns its index using `break`, and a program that sums only even numbers using `continue`.

**Acceptance Scenarios**:

1. **Given** a `while` loop with `break;`, **When** the break is reached, **Then** the loop exits immediately.
2. **Given** a `for` loop with `break;`, **When** the break is reached, **Then** the loop exits immediately.
3. **Given** a loop with `continue;`, **When** the continue is reached, **Then** the current iteration ends and the loop proceeds to the next.
4. **Given** nested loops with `break`, **When** `break` is used in the inner loop, **Then** only the inner loop exits.
5. **Given** `break` or `continue` used outside a loop, **When** compiled, **Then** the compiler reports an error.

---

### Edge Cases

- What happens when an array literal has more or fewer elements than the declared size? Compile error with clear message.
- What happens with zero-length arrays `[0]i32`? Allowed as a type but cannot be indexed.
- What happens when a struct field name conflicts with a keyword? Compile error.
- What happens with recursive structs (struct containing itself by value)? Compile error — requires pointer indirection.
- What happens with `break` or `continue` outside a loop? Compile error.
- What happens with `for` loop where range bounds are different integer types? Compile error.
- What happens with pointer arithmetic? Not supported — only `&`, `*`, and passing to functions.
- What happens when printing a struct? Prints in `StructName { field1: value1, field2: value2 }` format.
- What happens when dereferencing a null/dangling pointer? Undefined behavior (same as C); safety mechanisms deferred to future phase.

## Requirements *(mandatory)*

### Functional Requirements

**Arrays**
- **FR-001**: System MUST support fixed-size array types `[N]T` where N is a compile-time integer constant and T is any scalar or struct type
- **FR-002**: System MUST support array literal expressions `[expr1, expr2, ..., exprN]`
- **FR-003**: System MUST support array indexing `a[i]` for both reading and writing (on mutable arrays)
- **FR-004**: System MUST perform runtime bounds checking on array index operations
- **FR-005**: System MUST support arrays as function parameters and return values with value semantics

**Structs**
- **FR-006**: System MUST support struct type definitions with named, typed fields
- **FR-007**: System MUST support struct instantiation with named field values
- **FR-008**: System MUST support field access on struct values via dot notation
- **FR-009**: System MUST support methods on structs via explicit `self` parameter, callable with dot syntax
- **FR-010**: System MUST allocate structs on the stack by default with value semantics

**Pointers**
- **FR-011**: System MUST support pointer types `*T` for any type T
- **FR-012**: System MUST support address-of operator `&expr`
- **FR-013**: System MUST support dereference operator `*ptr` for reading
- **FR-014**: System MUST support dereference assignment `*ptr = expr` for writing
- **FR-015**: System MUST support automatic dereference for field access on struct pointers (`ptr.field` works like `(*ptr).field`)

**Strings and Output**
- **FR-016**: System MUST support a `str` type for immutable string values
- **FR-017**: System MUST support string literal expressions with double quotes and escape sequences (`\n`, `\t`, `\\`, `\"`)
- **FR-018**: System MUST provide `print(value)` built-in for stdout output without trailing newline
- **FR-019**: System MUST provide `println(value)` built-in for stdout output with trailing newline
- **FR-020**: `print` and `println` MUST accept all scalar types, strings, and structs

**For Loops and Control Flow**
- **FR-021**: System MUST support `for ident in start..end { body }` (end exclusive)
- **FR-022**: System MUST support `for ident in start..=end { body }` (end inclusive)
- **FR-023**: System MUST support `break` to exit the innermost enclosing loop
- **FR-024**: System MUST support `continue` to skip to the next iteration
- **FR-025**: System MUST report compile errors for `break`/`continue` outside loop bodies

**Type Inference**
- **FR-026**: System MUST support `name := expr;` for immutable inferred variable declarations
- **FR-027**: System MUST support `name :~= expr;` for mutable inferred variable declarations
- **FR-028**: Default inferred types MUST be `i32` for integers, `f64` for floats, `bool` for booleans

### Key Entities

- **Array Type `[N]T`**: Fixed-size, stack-allocated sequence of N elements of type T. N is a compile-time constant. Value semantics.
- **Struct Type**: Named composite type with named fields. Stack-allocated. Can have associated methods.
- **Pointer `*T`**: Memory address pointing to a value of type T. Supports `&`, `*`, and auto-deref for field access.
- **String `str`**: Immutable UTF-8 text value. Backed by pointer+length internally. Literals are compile-time constants.
- **Range `start..end` / `start..=end`**: Half-open or inclusive range for `for` loop iteration.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: A dot product function on two `[100]f64` arrays compiles and produces the correct result
- **SC-002**: A `Vec3` struct with `add`, `scale`, and `length` methods works correctly via value and pointer passing
- **SC-003**: `println` correctly prints integers, floats, booleans, strings, and structs to stdout
- **SC-004**: A matrix multiplication using nested `for` loops on `[3][3]f64` arrays produces correct results
- **SC-005**: All type errors produce clear messages with source locations
- **SC-006**: Compilation of a 200-line program using all new features completes in under 3 seconds
- **SC-007**: The test suite achieves at least 85% coverage on new compiler modules
- **SC-008**: Array-heavy programs compiled with `-O2` perform within 1.5x of equivalent C programs

## Assumptions

- Target platform remains x86-64 Linux only
- LLVM 18.x continues to be the code generation backend
- No garbage collector — all data in this phase is stack-allocated
- No generics — arrays and structs use concrete types only
- No dynamic dispatch — struct methods are statically dispatched
- No closures or first-class functions in this phase
- No module/import system — all code in a single source file
- String operations beyond literals and printing (concatenation, slicing) are deferred
- Pointer arithmetic is not supported — only `&`, `*`, and function passing
- Heap allocation (`alloc`/`free`) is deferred — all data is stack-allocated
- Array bounds checking is always enabled
- The entry point remains `fn main() -> i32`
