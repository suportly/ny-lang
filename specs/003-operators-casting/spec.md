# Feature Specification: Operators, Casting & Comments (Phase 3)

**Feature Branch**: `003-operators-casting`
**Created**: 2026-04-01
**Status**: Draft
**Input**: User description: "Ny Lang Phase 3: compound assignment operators (+=, -=, *=, /=, %=), bitwise operators (& | ^ << >> ~), type casting (expr as T for numeric conversions), and block comments (/* */)"

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Use Compound Assignment for Concise Mutation (Priority: P1)

A developer uses `+=`, `-=`, `*=`, `/=`, and `%=` operators to update mutable variables in-place, replacing verbose patterns like `x = x + 1` with `x += 1`. This is especially useful in loops and accumulators.

**Why this priority**: Compound assignment is the most frequently needed missing operator. Every loop accumulator (`sum = sum + val`) benefits from it. This is a small syntactic improvement with high daily impact.

**Independent Test**: Write a program that uses `x += 1` in a loop and returns the accumulated value.

**Acceptance Scenarios**:

1. **Given** `x :~ i32 = 0; x += 5;`, **When** compiled and run, **Then** `x` equals 5.
2. **Given** `x :~ i32 = 10; x -= 3;`, **When** compiled and run, **Then** `x` equals 7.
3. **Given** `x :~ i32 = 4; x *= 3;`, **When** compiled and run, **Then** `x` equals 12.
4. **Given** `x :~ i32 = 20; x /= 4;`, **When** compiled and run, **Then** `x` equals 5.
5. **Given** `x :~ i32 = 17; x %= 5;`, **When** compiled and run, **Then** `x` equals 2.
6. **Given** `x :~ f64 = 1.5; x += 2.5;`, **When** compiled and run, **Then** `x` equals 4.0.
7. **Given** an immutable variable `x : i32 = 5; x += 1;`, **When** compiled, **Then** the compiler reports an immutability error.
8. **Given** `arr :~ [3]i32 = [1, 2, 3]; arr[0] += 10;`, **When** compiled and run, **Then** `arr[0]` equals 11.

---

### User Story 2 - Perform Bitwise Operations (Priority: P1)

A developer uses bitwise operators to manipulate individual bits in integer values — essential for flags, masks, hash functions, and low-level data processing. The operators are: AND (`&`), OR (`|`), XOR (`^`), left shift (`<<`), right shift (`>>`), and bitwise NOT (`~`).

**Why this priority**: Bitwise operations are fundamental for systems programming and AI/ML (bit packing, SIMD masks, hash computations). Without them, Ny cannot handle any low-level data manipulation.

**Independent Test**: Write a program that sets, clears, and toggles bits in an integer using bitwise operators, and returns the result.

**Acceptance Scenarios**:

1. **Given** `0xFF & 0x0F`, **When** evaluated, **Then** the result is `0x0F` (AND masks lower nibble).
2. **Given** `0xF0 | 0x0F`, **When** evaluated, **Then** the result is `0xFF` (OR combines bits).
3. **Given** `0xFF ^ 0x0F`, **When** evaluated, **Then** the result is `0xF0` (XOR toggles bits).
4. **Given** `1 << 4`, **When** evaluated, **Then** the result is `16` (left shift).
5. **Given** `32 >> 2`, **When** evaluated, **Then** the result is `8` (right shift).
6. **Given** `~0`, **When** evaluated as `i32`, **Then** the result is `-1` (bitwise NOT flips all bits).
7. **Given** bitwise operators applied to a `bool` or `f64`, **When** compiled, **Then** the compiler reports a type error — bitwise operators require integer types.
8. **Given** `x :~ i32 = 0xFF; x &= 0x0F;`, **When** compiled and run, **Then** `x` equals `0x0F` (compound bitwise assignment).

---

### User Story 3 - Cast Between Numeric Types (Priority: P1)

A developer converts values between numeric types using `expr as T` syntax — for example, casting an `i32` to `i64` for wider arithmetic, or truncating an `f64` to `i32` for an exit code. This removes the "type silo" problem where different integer sizes are incompatible.

**Why this priority**: Without type casting, having 13 numeric types (i8-i128, u8-u128, f32, f64) creates friction. Functions requiring `i64` cannot accept `i32` values. Casting is essential for practical numeric programming.

**Independent Test**: Write a program that casts between integer sizes, between int and float, and verifies the results.

**Acceptance Scenarios**:

1. **Given** `42 as i64`, **When** evaluated, **Then** the result is an `i64` with value 42 (widening integer cast).
2. **Given** `1000 as i8`, **When** evaluated, **Then** the result is the truncated value (narrowing integer cast with wrapping).
3. **Given** `3.14 as i32`, **When** evaluated, **Then** the result is `3` (float-to-integer truncation).
4. **Given** `42 as f64`, **When** evaluated, **Then** the result is `42.0` (integer-to-float conversion).
5. **Given** `x : f32 = 1.5; x as f64`, **When** evaluated, **Then** the result is `1.5` as `f64` (float widening).
6. **Given** `x : f64 = 3.99; x as f32`, **When** evaluated, **Then** the result is approximately `3.99` as `f32` (float narrowing).
7. **Given** `true as i32`, **When** evaluated, **Then** the result is `1` (bool-to-integer cast).
8. **Given** a struct cast `v as i32`, **When** compiled, **Then** the compiler reports a type error — only numeric and bool casts are supported.
9. **Given** chained casts `(3.14 as i32) as i64`, **When** evaluated, **Then** the result is `3` as `i64`.

---

### User Story 4 - Write Block Comments (Priority: P2)

A developer uses `/* ... */` block comments to comment out multi-line sections of code or write inline documentation. Block comments can be nested, allowing developers to comment out regions that already contain block comments.

**Why this priority**: Block comments are a quality-of-life feature for documentation and debugging. Lower priority than operators and casting since single-line comments (`//`) already exist, but expected in any modern language.

**Independent Test**: Write a program with block comments (including nested ones) and verify it compiles and runs correctly, ignoring all commented content.

**Acceptance Scenarios**:

1. **Given** `/* this is a comment */ return 42;`, **When** compiled, **Then** the comment is ignored and the program returns 42.
2. **Given** a multi-line block comment spanning 5 lines, **When** compiled, **Then** all lines within `/* ... */` are ignored.
3. **Given** nested block comments `/* outer /* inner */ still outer */`, **When** compiled, **Then** all nested content is correctly ignored.
4. **Given** an unterminated block comment `/* no closing`, **When** compiled, **Then** the compiler reports a clear error indicating the comment was not closed.
5. **Given** `x := 5; /* inline */ y := 10;`, **When** compiled, **Then** both `x` and `y` are correctly declared.

---

### Edge Cases

- What happens with `x += 1` on an immutable variable? Compile error — same as `x = x + 1` on immutable.
- What happens with compound assignment on array indices `arr[i] += 1`? Works — desugars to `arr[i] = arr[i] + 1`.
- What happens with compound assignment on struct fields `v.x += 1.0`? Works — desugars to `v.x = v.x + 1.0`.
- What happens with compound assignment on dereferenced pointers `*p += 1`? Works — desugars to `*p = *p + 1`.
- What happens with `&` in infix position vs prefix? Prefix `&x` is address-of. Infix `a & b` is bitwise AND. Disambiguated by parser context.
- What happens with `*` in prefix position vs infix? Prefix `*p` is dereference. Infix `a * b` is multiplication. Already handled.
- What happens casting a negative signed integer to an unsigned type? Wrapping behavior (bit pattern preserved), same as C.
- What happens with deeply nested block comments? Supported — nesting depth is tracked.
- What happens with `/* */` inside a string literal? Not treated as a comment — string contents are preserved verbatim.
- What happens with shift amounts larger than the bit width? Undefined behavior, same as C/LLVM semantics.

## Requirements *(mandatory)*

### Functional Requirements

**Compound Assignment**
- **FR-001**: System MUST support compound assignment operators: `+=`, `-=`, `*=`, `/=`, `%=` on mutable numeric variables
- **FR-002**: System MUST support compound bitwise assignment operators: `&=`, `|=`, `^=`, `<<=`, `>>=` on mutable integer variables
- **FR-003**: Compound assignment MUST work on all assignment targets: variables, array indices (`arr[i] += 1`), struct fields (`v.x += 1.0`), and pointer dereferences (`*p += 1`)
- **FR-004**: Compound assignment on immutable variables MUST produce a compile error

**Bitwise Operators**
- **FR-005**: System MUST support binary bitwise operators: `&` (AND), `|` (OR), `^` (XOR), `<<` (left shift), `>>` (right shift)
- **FR-006**: System MUST support unary bitwise NOT operator: `~`
- **FR-007**: Bitwise operators MUST require integer operands — applying them to floats, bools, strings, structs, or pointers MUST produce a compile error
- **FR-008**: Right shift MUST be arithmetic (sign-extending) for signed integers and logical (zero-filling) for unsigned integers
- **FR-009**: Bitwise operator precedence MUST follow C conventions: `~` highest, then `<< >>`, then `&`, then `^`, then `|`

**Type Casting**
- **FR-010**: System MUST support type casting syntax `expr as T` where T is a scalar type
- **FR-011**: System MUST support integer-to-integer casts: widening (sign/zero extension) and narrowing (truncation)
- **FR-012**: System MUST support integer-to-float and float-to-integer conversions
- **FR-013**: System MUST support float-to-float conversions (f32 to f64 and f64 to f32)
- **FR-014**: System MUST support bool-to-integer cast (true=1, false=0)
- **FR-015**: System MUST reject casts involving non-scalar types (structs, arrays, pointers, strings) with a compile error
- **FR-016**: The `as` operator MUST have higher precedence than arithmetic operators (binds tightly to the left operand)

**Block Comments**
- **FR-017**: System MUST support block comments delimited by `/*` and `*/`
- **FR-018**: Block comments MUST support nesting (`/* /* inner */ outer */`)
- **FR-019**: Unterminated block comments MUST produce a compile error with a clear message
- **FR-020**: Block comments inside string literals MUST NOT be treated as comments

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: A loop using `sum += arr[i]` produces the same result as the equivalent `sum = sum + arr[i]` code
- **SC-002**: A bit manipulation program (set/clear/toggle flags) compiles and produces correct results for all bitwise operators
- **SC-003**: A program casting `i32` to `i64`, `f64` to `i32`, and `i32` to `f64` produces correct results for all conversions
- **SC-004**: All type errors for invalid casts and invalid bitwise operands produce clear error messages with source locations
- **SC-005**: Programs with block comments (including nested) compile correctly, ignoring all commented content
- **SC-006**: All existing Phase 1 and Phase 2 tests continue to pass (no regressions)
- **SC-007**: Compilation time for a 200-line program remains under 3 seconds

## Assumptions

- Compound assignment is syntactic sugar — `x += y` is semantically identical to `x = x + y`
- Compound assignment operators are not expressions (they are statements, no value is produced)
- Bitwise operators follow C/Rust precedence conventions
- The `&` token is disambiguated by context: prefix = address-of, infix = bitwise AND (existing `&&` remains logical AND)
- The `|` token is the new bitwise OR; existing `||` remains logical OR
- Type casting follows C semantics for truncation and wrapping behavior
- No implicit type coercion — all conversions require explicit `as` syntax
- Block comments nest (unlike C, but like Rust and Swift)
- The `~` operator is unary prefix, distinct from `!` (logical NOT)
- `as` has the same precedence as a postfix operator (higher than arithmetic), matching Rust semantics
- No compound shift-assign (`<<=`, `>>=`) is deferred if too complex — included per FR-002
