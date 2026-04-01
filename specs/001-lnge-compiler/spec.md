# Feature Specification: LNGE Compiler MVP (Phases 0-3)

**Feature Branch**: `001-lnge-compiler`
**Created**: 2026-04-01
**Status**: Draft
**Input**: User description: "LNGE - Low-level language for AI/ML with extreme performance, Rust compiler with LLVM backend"

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Compile and Run Scalar Programs (Priority: P1)

A developer writes a simple LNGE program with scalar arithmetic, functions, and control flow (e.g., a Fibonacci function). They invoke the LNGE compiler CLI, which produces a native executable. Running the executable produces the correct result.

**Why this priority**: This is the foundational MVP. Without compiling and executing scalar code, no further language features can be built or tested. This proves the entire pipeline works end-to-end: source code -> lexer -> parser -> semantic analysis -> LLVM codegen -> native binary.

**Independent Test**: Can be fully tested by writing `fn main() -> i32 { return fibonacci(40); }`, compiling it with `lnge build hello.lnge`, and verifying the output executable returns the correct Fibonacci result (102334155).

**Acceptance Scenarios**:

1. **Given** a valid LNGE source file with a `main` function returning an i32, **When** the user runs `lnge build <file>`, **Then** the compiler produces a native executable that exits with the correct return code.
2. **Given** a LNGE source file with a recursive `fibonacci(40)` function, **When** compiled and executed, **Then** the program returns 102334155 and completes within a reasonable time.
3. **Given** a LNGE source file with arithmetic expressions (`+`, `-`, `*`, `/`, `%`), **When** compiled, **Then** the executable produces correct results for all operations.

---

### User Story 2 - Get Clear Error Messages on Invalid Code (Priority: P2)

A developer writes LNGE code with syntax errors or type mismatches. The compiler reports errors with precise source locations (file, line, column), a highlighted code snippet, and a human-readable message explaining what went wrong.

**Why this priority**: Good error reporting is essential for developer productivity and adoption. Without clear errors, even correct language features become unusable because developers cannot diagnose their mistakes.

**Independent Test**: Can be tested by feeding the compiler intentionally malformed source files and verifying that error messages include the correct line/column, relevant source snippet, and an understandable description of the problem.

**Acceptance Scenarios**:

1. **Given** a source file with a syntax error (e.g., missing semicolon), **When** compiled, **Then** the compiler outputs an error message pointing to the exact line and column with a code snippet.
2. **Given** a source file with a type mismatch (e.g., assigning a bool to an i32 variable), **When** compiled, **Then** the compiler reports a type error with the expected and found types.
3. **Given** a source file referencing an undeclared variable, **When** compiled, **Then** the compiler reports "undeclared variable" with the name and location.

---

### User Story 3 - Use Variables, Mutability, and Control Flow (Priority: P1)

A developer writes LNGE programs using immutable variables (`:` syntax), mutable variables (`:~` syntax), if/else conditionals, and while loops. The compiler correctly enforces immutability and generates efficient native code for all control flow constructs.

**Why this priority**: Variables and control flow are fundamental building blocks needed by every non-trivial program. They are required for the Fibonacci benchmark and all subsequent language features.

**Independent Test**: Can be tested by writing programs that use mutable counters in loops, conditional branching, and verifying that reassigning an immutable variable produces a compile error.

**Acceptance Scenarios**:

1. **Given** a program declaring `x : i32 = 5;` and attempting `x = 10;`, **When** compiled, **Then** the compiler reports an immutability error.
2. **Given** a program declaring `x :~ i32 = 0;` and using `x = x + 1;` in a while loop, **When** compiled and run, **Then** the variable is correctly mutated and the loop terminates with the expected value.
3. **Given** a program with nested if/else branches returning different values, **When** compiled and run, **Then** the correct branch is taken and the right value is returned.

---

### User Story 4 - Define and Call Functions (Priority: P1)

A developer defines multiple functions with typed parameters and return types, calls them from `main`, and the compiler resolves all function references, checks argument types, and generates correct call/return sequences.

**Why this priority**: Functions are required for code organization and for the Fibonacci benchmark (recursive calls). They are the primary abstraction mechanism in the MVP.

**Independent Test**: Can be tested by writing a program with multiple functions calling each other, including recursive calls, and verifying correct return values.

**Acceptance Scenarios**:

1. **Given** a program with `fn add(a: i32, b: i32) -> i32 = a + b;` called from main, **When** compiled and run, **Then** the result is correct.
2. **Given** a recursive function `fn fib(n: i32) -> i32`, **When** compiled and called with `fib(10)`, **Then** the result is 55.
3. **Given** a function called with wrong argument types, **When** compiled, **Then** the compiler reports a type mismatch error.

---

### Edge Cases

- What happens when the source file is empty? The compiler reports "no main function found."
- What happens when the source file does not exist? The compiler reports a file-not-found error with the path.
- What happens with integer overflow? The MVP uses wrapping arithmetic consistent with low-level language semantics.
- What happens with division by zero? Behavior follows the target platform (hardware trap on x86).
- What happens with deeply recursive functions (stack overflow)? Behavior follows OS defaults; no special handling in MVP.
- What happens with mutually recursive functions? The compiler handles forward references for function declarations.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: System MUST tokenize LNGE source files into a stream of typed tokens with source location spans
- **FR-002**: System MUST support all scalar types: i8, i16, i32, i64, i128, u8, u16, u32, u64, u128, f32, f64, bool
- **FR-003**: System MUST parse expressions using operator precedence (Pratt parsing) including arithmetic (`+`, `-`, `*`, `/`, `%`), comparison (`==`, `!=`, `<`, `>`, `<=`, `>=`), logical (`&&`, `||`, `!`), and unary (`-`) operators
- **FR-004**: System MUST parse function definitions with typed parameters and return types
- **FR-005**: System MUST parse variable declarations with both immutable (`:`) and mutable (`:~`) syntax
- **FR-006**: System MUST parse control flow: if/else expressions, while loops, and block expressions with `{ }`
- **FR-007**: System MUST perform name resolution, rejecting undeclared identifiers
- **FR-008**: System MUST perform type checking, ensuring all expressions have compatible types
- **FR-009**: System MUST enforce immutability — reassignment of immutable variables MUST produce a compile error
- **FR-010**: System MUST generate LLVM IR for all supported scalar operations and control flow
- **FR-011**: System MUST produce native executables via LLVM for the host platform
- **FR-012**: System MUST provide error messages with source locations (file, line, column) and code snippets
- **FR-013**: System MUST provide a CLI interface accepting source file paths and producing executables
- **FR-014**: System MUST support function calls including recursive calls with correct stack frame management
- **FR-015**: System MUST support compile-time constants (`::` syntax) evaluated at compile time
- **FR-016**: System MUST support single-line comments (`//`)

### Key Entities

- **Token**: A lexical unit with a type (keyword, identifier, literal, operator, punctuation), a value, and a source span
- **AST Node**: A node in the abstract syntax tree representing an expression, statement, item (function), or type annotation
- **Type**: A representation of LNGE types used during semantic analysis — scalar types, function signatures, and the unit type
- **LLVM Module**: The generated LLVM intermediate representation containing functions, basic blocks, and instructions

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: A recursive Fibonacci(40) program compiles and runs producing the correct result (102334155)
- **SC-002**: Compilation of a 100-line scalar program completes in under 2 seconds on standard hardware
- **SC-003**: The compiled Fibonacci(40) executable with -O2 runs within 1.2x of equivalent C code compiled with gcc -O2 on x86-64 Linux
- **SC-007**: The compiled Fibonacci(40) executable with -O2 runs faster than equivalent Go program compiled with `go build`
- **SC-004**: All syntax errors and type errors produce messages that include the exact source line and column number
- **SC-005**: The compiler rejects 100% of programs with type mismatches, undeclared variables, and immutability violations
- **SC-006**: The test suite (lexer, parser, semantic analysis, codegen, integration) achieves at least 90% coverage on core logic

## Assumptions

- The target platform for MVP is x86-64 Linux; cross-compilation is out of scope for this phase
- LLVM 18.x is available on the build system
- The MVP does not include arrays, pointers, slices, SIMD, tensors, or GPU features — these are subsequent phases
- The MVP does not include a standard library — only built-in scalar operations and user-defined functions
- String types and string literals are out of scope for MVP
- The entry point is always a function named `main` returning `i32`
- No module system or imports in MVP — all code is in a single source file
- Error recovery in the parser is best-effort; the compiler may stop at the first error in MVP
