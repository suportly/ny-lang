# Feature Specification: Strings, Enums & Tuples (Phase 4)

**Feature Branch**: `004-strings-enums-tuples`
**Created**: 2026-04-01
**Status**: Draft
**Input**: User description: "Ny Lang Phase 4: string operations (len, concat, compare, substring), enums with match expressions, tuple types with multi-return values and destructuring"

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Manipulate Strings (Priority: P1)

A developer works with strings beyond just printing them — getting the length, comparing two strings, concatenating strings, and extracting substrings. String comparison with `==` and `!=` works naturally. A `len()` method returns the byte length. Concatenation with `+` creates a new string. Substring extraction with `substr(start, end)` returns a slice.

**Why this priority**: Currently strings are read-only literals that can only be printed. Without string operations, Ny cannot process text input, build output messages, or implement any text-based logic. This is the #1 gap for practical programs.

**Independent Test**: Write a program that concatenates two strings, checks their length, compares them, and prints the result.

**Acceptance Scenarios**:

1. **Given** `s : str = "hello"; println(s.len());`, **When** compiled and run, **Then** stdout contains `5`.
2. **Given** `a : str = "hello"; b : str = " world"; c := a + b; println(c);`, **When** compiled and run, **Then** stdout contains `hello world`.
3. **Given** `a : str = "abc"; b : str = "abc"; println(a == b);`, **When** compiled and run, **Then** stdout contains `true`.
4. **Given** `a : str = "abc"; b : str = "xyz"; println(a != b);`, **When** compiled and run, **Then** stdout contains `true`.
5. **Given** `s : str = "hello world"; sub := s.substr(0, 5); println(sub);`, **When** compiled and run, **Then** stdout contains `hello`.
6. **Given** `s : str = ""; println(s.len());`, **When** compiled and run, **Then** stdout contains `0`.

---

### User Story 2 - Define and Use Enums (Priority: P1)

A developer defines enumeration types with named variants (without data payloads in this phase) and uses them for type-safe state representation. Enum values can be compared with `==` and `!=`, passed to functions, and stored in variables.

**Why this priority**: Enums are essential for representing states, error codes, options, and categories. Without them, developers use magic integers, which is error-prone and unreadable. Enums with match expressions replace verbose if/else chains.

**Independent Test**: Define a `Color` enum with `Red`, `Green`, `Blue` variants, match on a value, and return different results per variant.

**Acceptance Scenarios**:

1. **Given** `enum Color { Red, Green, Blue }`, **When** compiled, **Then** the enum type is recognized with three variants.
2. **Given** `c : Color = Color::Red;`, **When** compiled, **Then** the variable holds the `Red` variant.
3. **Given** `c == Color::Red`, **When** evaluated, **Then** the result is `true`.
4. **Given** `c != Color::Blue`, **When** evaluated, **Then** the result is `true`.
5. **Given** an enum passed to a function `fn describe(c: Color) -> i32`, **When** called, **Then** the function receives the correct variant.
6. **Given** `println(c);`, **When** compiled and run, **Then** stdout contains the variant name (e.g., `Red`).

---

### User Story 3 - Match Expressions on Enums (Priority: P1)

A developer uses `match` expressions to branch on enum variants, replacing long if/else chains with concise, exhaustive pattern matching. The compiler enforces that all variants are covered (or a default `_` arm is provided).

**Why this priority**: Match is the natural companion to enums and is strictly more powerful than switch/case. Exhaustiveness checking catches bugs at compile time when new variants are added.

**Independent Test**: Write a match expression that maps each Color variant to an integer and returns the result.

**Acceptance Scenarios**:

1. **Given** `match c { Color::Red => 1, Color::Green => 2, Color::Blue => 3 }`, **When** `c` is `Red`, **Then** the result is `1`.
2. **Given** a match expression used as a value `x := match c { ... };`, **When** compiled, **Then** the match result is assigned to `x`.
3. **Given** a match missing a variant and no `_` default, **When** compiled, **Then** the compiler reports a non-exhaustive match error.
4. **Given** `match c { Color::Red => 1, _ => 0 }`, **When** `c` is `Green`, **Then** the result is `0` (default arm).
5. **Given** match on an integer `match x { 0 => "zero", 1 => "one", _ => "other" }`, **When** `x` is `1`, **Then** the result is `"one"`.
6. **Given** match arms with block bodies `Color::Red => { println("red"); 1 }`, **When** matched, **Then** the block executes and the last expression is the result.

---

### User Story 4 - Use Tuple Types and Multi-Return (Priority: P1)

A developer uses tuple types to group multiple values without defining a named struct, and functions can return tuples to provide multiple results (e.g., value + error). Tuple destructuring allows binding each element to a separate variable.

**Why this priority**: Tuples enable Go-style multi-return `(val, err)` without defining a struct for every return combination. This is the foundation for error handling patterns in Phase 5.

**Independent Test**: Write a function returning `(i32, bool)` and destructure the result at the call site.

**Acceptance Scenarios**:

1. **Given** `t : (i32, bool) = (42, true);`, **When** compiled, **Then** `t` holds a tuple of `(42, true)`.
2. **Given** `t.0`, **When** evaluated, **Then** the result is `42` (first element).
3. **Given** `t.1`, **When** evaluated, **Then** the result is `true` (second element).
4. **Given** `fn divide(a: i32, b: i32) -> (i32, bool)` that returns `(a / b, true)` on success and `(0, false)` on division by zero, **When** called, **Then** the correct tuple is returned.
5. **Given** `(result, ok) := divide(10, 3);`, **When** compiled, **Then** `result` is `3` and `ok` is `true` (destructuring).
6. **Given** `println(t);`, **When** compiled and run, **Then** stdout contains `(42, true)`.
7. **Given** a nested tuple `((i32, i32), bool)`, **When** compiled, **Then** the nested structure is supported.

---

### Edge Cases

- What happens when concatenating a string with a non-string type? Compile error — only `str + str` is supported (no implicit conversion).
- What happens with `substr` out of bounds? Returns empty string for out-of-range, or truncates to available length.
- What happens with an empty enum `enum Empty {}`? Compile error — enums must have at least one variant.
- What happens with duplicate enum variant names? Compile error.
- What happens with a match on a non-enum, non-integer type (e.g., float)? Compile error — match supports enums and integers only.
- What happens with an empty tuple `()`? This is the `Unit` type, already supported.
- What happens with a single-element tuple `(i32,)`? Treated as a 1-tuple (distinct from parenthesized expression `(i32)`), disambiguated by trailing comma.
- What happens when string memory from concatenation is not freed? Memory leak — heap cleanup is deferred to Phase 5 (defer/free).

## Requirements *(mandatory)*

### Functional Requirements

**String Operations**
- **FR-001**: System MUST support `str.len()` method returning the byte length as `i64`
- **FR-002**: System MUST support string concatenation with `+` operator producing a new `str` value
- **FR-003**: System MUST support string comparison with `==` and `!=` operators returning `bool`
- **FR-004**: System MUST support `str.substr(start: i64, end: i64)` method returning a substring
- **FR-005**: String concatenation MUST allocate memory for the new string (introduces heap allocation for strings)
- **FR-006**: String operations MUST preserve UTF-8 encoding

**Enums**
- **FR-007**: System MUST support enum type definitions with named variants: `enum Name { Variant1, Variant2, ... }`
- **FR-008**: System MUST support enum variant access via `EnumName::VariantName` syntax
- **FR-009**: System MUST support equality comparison (`==`, `!=`) on enum values
- **FR-010**: System MUST support enum values as function parameters and return types
- **FR-011**: System MUST support printing enum values (displays the variant name)

**Match Expressions**
- **FR-012**: System MUST support `match expr { pattern => result, ... }` as an expression that returns a value
- **FR-013**: Match MUST support enum variant patterns: `EnumName::Variant => expr`
- **FR-014**: Match MUST support integer literal patterns: `0 => expr, 1 => expr`
- **FR-015**: Match MUST support a wildcard/default pattern: `_ => expr`
- **FR-016**: Match on enums MUST enforce exhaustiveness — all variants must be covered or a `_` arm must be present
- **FR-017**: Match arms MUST support both expression bodies and block bodies

**Tuples**
- **FR-018**: System MUST support tuple type annotations: `(T1, T2, ...)` for 2 or more elements
- **FR-019**: System MUST support tuple literal expressions: `(expr1, expr2, ...)`
- **FR-020**: System MUST support tuple element access by index: `tuple.0`, `tuple.1`, etc.
- **FR-021**: System MUST support functions returning tuple types
- **FR-022**: System MUST support tuple destructuring in variable declarations: `(a, b) := expr;`
- **FR-023**: System MUST support printing tuples (displays as `(v1, v2, ...)`)

### Key Entities

- **String `str`**: Immutable UTF-8 text value. `len()` returns byte length, `substr()` returns a view, `+` allocates a new string.
- **Enum Type**: Named type with a fixed set of named variants. Each variant maps to an integer discriminant. No data payloads in this phase.
- **Match Expression**: Pattern-matching expression that evaluates to a value. Arms consist of pattern => body pairs.
- **Tuple Type `(T1, T2, ...)`**: Anonymous product type holding 2+ values. Accessed by index (`.0`, `.1`). Functions can return tuples for multi-return.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: A text processing program using string concat, len, compare, and substr compiles and produces correct results
- **SC-002**: An enum-based state machine with match expressions compiles and correctly dispatches on all variants
- **SC-003**: A function returning `(i32, bool)` can be called and destructured at the call site
- **SC-004**: The compiler reports non-exhaustive match errors when enum variants are missing and no default arm exists
- **SC-005**: All type errors for invalid operations (string + int, match on float, etc.) produce clear messages
- **SC-006**: All existing Phase 1-3 tests continue to pass (no regressions)
- **SC-007**: The test suite includes at least 5 new compile-and-run tests covering all new features

## Assumptions

- Enums in this phase are C-style (no data payloads). Data-carrying variants (tagged unions) are deferred to a future phase.
- String concatenation introduces the first heap allocation in Ny (via malloc). The allocated memory is not automatically freed — manual memory management is deferred to Phase 5 (defer/free).
- `substr` returns a view into the original string (no copy, just pointer+length adjustment). This means the original string must outlive the substring — a constraint documented but not enforced (no lifetime system yet).
- Match supports enums and integer literals only. String pattern matching is deferred.
- Tuple indexing uses `.0`, `.1` syntax (disambiguated from float literals and field access by context).
- Tuple destructuring `(a, b) := expr;` creates immutable bindings. Mutable destructuring `(a, b) :~= expr;` is also supported.
- The `+` operator for strings is only `str + str`. No implicit conversions from other types.
- Match is always an expression (returns a value). To use as a statement, wrap in an expression statement: `match x { ... };`.
