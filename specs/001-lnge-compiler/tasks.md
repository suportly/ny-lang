# Tasks: LNGE Compiler MVP

**Input**: Design documents from `/specs/001-lnge-compiler/`
**Prerequisites**: plan.md, spec.md, research.md, data-model.md, contracts/cli-contract.md

**Organization**: Tasks are grouped by user story. User stories are ordered so each phase delivers a testable increment of the compiler pipeline.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (US1, US2, US3, US4)

---

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: Project initialization, directory structure, and dependency configuration

- [x] T001 Create Cargo.toml with dependencies: inkwell (features = ["llvm18-0"]), codespan-reporting, clap (features = ["derive"]) in Cargo.toml
- [x] T002 Create project directory structure: src/{main.rs, lib.rs, lexer/, parser/, semantic/, codegen/, diagnostics/, common/} and tests/{integration/, unit/, fixtures/valid/, fixtures/invalid/}
- [x] T003 [P] Create stub src/main.rs with clap CLI skeleton (build subcommand, file argument, --output, --emit, --opt-level options) per contracts/cli-contract.md
- [x] T004 [P] Create src/lib.rs re-exporting all modules with placeholder mod declarations

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Shared type definitions and infrastructure that ALL user stories depend on

**CRITICAL**: No user story work can begin until this phase is complete

- [x] T005 [P] Implement Span type (file_id, start, end byte offsets) and source location utilities in src/common/span.rs
- [x] T006 [P] Implement LngeType enum (I8, I16, I32, I64, I128, U8, U16, U32, U64, U128, F32, F64, Bool, Unit, Function) with display formatting in src/common/types.rs
- [x] T007 [P] Create src/common/mod.rs re-exporting Span, LngeType, and shared error types (CompileError with message, span, ErrorKind enum)
- [x] T008 [P] Implement TokenKind enum with all variants (literals, identifiers, keywords, operators, punctuation, Eof) and Token struct (kind + span) in src/lexer/token.rs
- [x] T009 [P] Implement AST node types: Program, Item (FunctionDef), Param, Stmt (VarDecl, ConstDecl, Assign, ExprStmt, Return, While), Expr (Literal, Ident, BinOp, UnaryOp, Call, If, Block), Mutability, BinOp, UnaryOp, LitValue, TypeAnnotation enums in src/parser/ast.rs
- [x] T010 [P] Implement binding power table: prefix_bp() and infix_bp() functions per research.md precedence table in src/parser/precedence.rs
- [x] T011 [P] Implement diagnostics module: DiagnosticEmitter wrapping codespan-reporting SimpleFiles, Diagnostic construction from CompileError, colored terminal output in src/diagnostics/mod.rs
- [x] T012 [P] Implement LNGE-to-LLVM type mapping: lnge_to_llvm() function mapping each LngeType variant to the corresponding inkwell BasicTypeEnum in src/codegen/types.rs

**Checkpoint**: All shared types defined. Pipeline modules can now be implemented.

---

## Phase 3: User Story 4 - Define and Call Functions (Priority: P1) MVP

**Goal**: Build the minimal end-to-end compiler pipeline. A program with function definitions, parameters, return statements, and function calls compiles to a native executable.

**Independent Test**: Compile `fn main() -> i32 { return 42; }` with `lnge build`, run the binary, verify exit code is 42. Compile a program with `fn add(a: i32, b: i32) -> i32 { return a + b; } fn main() -> i32 { return add(20, 22); }` and verify exit code 42.

### Implementation for User Story 4

- [x] T013 [US4] Implement Lexer struct: new(source) constructor, next_token() method that scans characters and produces Tokens. Handle: whitespace skipping, single-line comments (//), identifiers/keywords (fn, return, if, else, while, true, false), integer literals, punctuation ({, }, (, ), ;, ,), arrow (->), colon (:), colon-tilde (:~), colon-colon (::), assignment (=) in src/lexer/mod.rs
- [x] T014 [US4] Extend Lexer with operator scanning: +, -, *, /, %, ==, !=, <, >, <=, >=, &&, ||, ! and float literal support (digits.digits) in src/lexer/mod.rs
- [x] T015 [US4] Implement Parser struct: new(tokens) constructor, parse_program() → Program. Implement parse_function() for function definitions with typed parameters and return type, parse_block() for { stmt* expr? } blocks, parse_return_stmt() in src/parser/mod.rs
- [x] T016 [US4] Implement parse_expr() using Pratt parsing: parse_expr_bp(min_bp) with prefix dispatch (literals, identifiers, unary ops, parenthesized expressions) and infix dispatch (binary operators) using precedence from src/parser/precedence.rs. Implement parse_call_args() for function call expressions in src/parser/mod.rs
- [x] T017 [US4] Implement Resolver struct for name resolution: push_scope/pop_scope, declare_symbol, resolve_name with scope chain lookup. Walk AST to resolve all identifiers, report undeclared variable/function errors. Register function signatures in global scope before resolving bodies (forward references) in src/semantic/resolver.rs
- [x] T018 [US4] Implement TypeChecker struct: check_function() validates return type matches body, check_expr() infers and validates expression types, check_call() validates argument count and types against function signature. Return typed information for codegen in src/semantic/typechecker.rs
- [x] T019 [US4] Create semantic analysis coordinator: analyze(program) runs Resolver then TypeChecker, collects all errors in src/semantic/mod.rs
- [x] T020 [US4] Implement CodeGen struct using inkwell: create LLVM Context/Module/Builder, compile_program() iterates items, compile_function() creates LLVM function with correct signature, compile_block/compile_stmt/compile_expr generates LLVM IR for literals, function calls, and return statements. Handle i32 main function as entry point in src/codegen/mod.rs
- [x] T021 [US4] Implement object file emission and linking: emit_object_file() using TargetMachine::write_to_file(FileType::Object), link_executable() shelling out to cc via std::process::Command in src/codegen/mod.rs
- [x] T022 [US4] Wire up full pipeline in src/main.rs: read source file → Lexer::new → Parser::new → parse_program → semantic::analyze → CodeGen::compile → emit → link. Handle errors with DiagnosticEmitter, set exit codes per CLI contract (0=success, 1=compile error, 2=I/O error)
- [x] T023 [US4] Create test fixture tests/fixtures/valid/return_42.lnge (`fn main() -> i32 { return 42; }`) and tests/fixtures/valid/function_call.lnge (add function + main calling it). Create integration test in tests/integration/compile_run.rs that compiles fixtures and verifies binary exit codes

**Checkpoint**: Minimal compiler works end-to-end. `fn main() -> i32 { return 42; }` compiles and runs.

---

## Phase 4: User Story 1 - Compile and Run Scalar Programs (Priority: P1)

**Goal**: Extend the pipeline with full scalar arithmetic expressions. All arithmetic, comparison, logical, and unary operators work on all scalar types.

**Independent Test**: Compile a program with arithmetic expressions (`2 + 3 * 4`, `10 / 3`, `7 % 2`), run the binary, verify correct results via exit code.

### Implementation for User Story 1

- [x] T024 [P] [US1] Extend CodeGen::compile_expr() with LLVM IR for binary operators: build_int_add/sub/mul (signed div/rem for signed, unsigned for unsigned), build_float_add/sub/mul/div, build_int_compare for all comparison ops, build_and/build_or for logical ops in src/codegen/mod.rs
- [x] T025 [P] [US1] Extend CodeGen::compile_expr() with LLVM IR for unary operators: build_int_neg for negation, build_not for logical not in src/codegen/mod.rs
- [x] T026 [US1] Extend TypeChecker: validate arithmetic operators require numeric types, comparison operators produce bool, logical operators require bool operands, unary negation requires numeric, unary not requires bool in src/semantic/typechecker.rs
- [x] T027 [US1] Add support for all scalar type annotations in parser: parse i8, i16, i32, i64, i128, u8, u16, u32, u64, u128, f32, f64, bool as TypeAnnotation::Named variants. Map type name strings to LngeType in resolver in src/parser/mod.rs and src/semantic/resolver.rs
- [x] T028 [US1] Create test fixtures: tests/fixtures/valid/arithmetic.lnge (mixed arithmetic returning a computed value), tests/fixtures/valid/comparisons.lnge (comparison operators), tests/fixtures/valid/boolean_logic.lnge (logical operators). Add integration tests verifying correct exit codes in tests/integration/compile_run.rs

**Checkpoint**: Full scalar arithmetic works. Programs with complex expressions compile and run correctly.

---

## Phase 5: User Story 3 - Variables, Mutability, and Control Flow (Priority: P1)

**Goal**: Add variable declarations (immutable and mutable), immutability enforcement, if/else conditionals, while loops, and block expressions.

**Independent Test**: Compile the Fibonacci(40) program, run the binary, verify it returns 102334155 (via process exit code modulo 256 = 155, or capture stdout if using a print mechanism). Verify that reassigning an immutable variable produces a compile error.

### Implementation for User Story 3

- [x] T029 [US3] Implement parse_stmt() extensions in parser: parse VarDecl (name : type = expr; for immutable and name :~ type = expr; for mutable), parse ConstDecl (name :: type = expr;), parse Assign (name = expr;), parse while loops (while expr { block }). Distinguish statement vs expression-statement in src/parser/mod.rs
- [x] T030 [US3] Implement parse_if_expr() in parser: parse if condition { block } else { block } as an Expr::If node. Support if/else chains. Handle if as expression (returns value from branches) in src/parser/mod.rs
- [x] T031 [US3] Extend Resolver with variable handling: declare variables in current scope with mutability tracking, resolve variable references, report duplicate declarations within same scope in src/semantic/resolver.rs
- [x] T032 [US3] Implement immutability enforcement in TypeChecker: on Assign statements, check that the target variable is declared as Mutable. Produce compile error "cannot assign to immutable variable" with source location for immutable reassignment in src/semantic/typechecker.rs
- [x] T033 [US3] Extend TypeChecker for control flow: validate if condition is bool, check that both branches of if/else have compatible types when used as expression, validate while condition is bool in src/semantic/typechecker.rs
- [x] T034 [US3] Implement CodeGen for variables: use build_alloca for local variables, build_store for initialization and assignment, build_load for variable reads. Handle mutable variables with stack allocation in src/codegen/mod.rs
- [x] T035 [US3] Implement CodeGen for control flow: build_conditional_branch for if/else with then/else/merge basic blocks, build_unconditional_branch and phi nodes for if-as-expression, while loops with condition/body/exit blocks in src/codegen/mod.rs
- [x] T036 [US3] Implement compile-time constant evaluation for ConstDecl (:: syntax): evaluate constant integer/float/bool expressions at compile time, store as LLVM constants rather than allocas in src/codegen/mod.rs
- [x] T037 [US3] Create test fixtures: tests/fixtures/valid/variables.lnge (mutable counter in loop), tests/fixtures/valid/fibonacci.lnge (recursive fibonacci), tests/fixtures/valid/control_flow.lnge (nested if/else). Create tests/fixtures/invalid/immutable_assign.lnge (reassign immutable var). Add integration tests in tests/integration/compile_run.rs and tests/integration/error_tests.rs

**Checkpoint**: Fibonacci(40) compiles and runs. Immutability is enforced. All P1 stories complete.

---

## Phase 6: User Story 2 - Clear Error Messages (Priority: P2)

**Goal**: All compiler errors include precise source locations (file, line, column), highlighted code snippets, and human-readable descriptions.

**Independent Test**: Feed the compiler files with syntax errors, type mismatches, and undeclared variables. Verify each error message includes the correct file:line:column, a code snippet with the error highlighted, and a descriptive message.

### Implementation for User Story 2

- [x] T038 [US2] Enhance Lexer error reporting: produce CompileError with precise spans for unexpected characters, unterminated tokens, and invalid number literals in src/lexer/mod.rs
- [x] T039 [US2] Enhance Parser error reporting: produce CompileError with spans for expected-but-found token errors, missing semicolons, malformed function signatures, unclosed braces/parens in src/parser/mod.rs
- [x] T040 [US2] Enhance Resolver error messages: include "did you mean?" suggestions for similar identifiers (Levenshtein distance), include the declaration location for duplicate variable errors in src/semantic/resolver.rs
- [x] T041 [US2] Enhance TypeChecker error messages: include expected vs found types in type mismatch errors, include function signature in argument count/type errors, include "declared as immutable here" secondary label for immutability violations in src/semantic/typechecker.rs
- [x] T042 [US2] Improve DiagnosticEmitter: add secondary labels support, add notes/suggestions support, ensure all error codes follow E### format per CLI contract in src/diagnostics/mod.rs
- [x] T043 [US2] Handle edge case errors: empty source file → "no main function found", nonexistent file → file-not-found with path (exit code 2), file read permission error → clear message (exit code 2) in src/main.rs
- [x] T044 [US2] Create error test fixtures: tests/fixtures/invalid/syntax_error.lnge (missing semicolon), tests/fixtures/invalid/type_mismatch.lnge (bool assigned to i32), tests/fixtures/invalid/undeclared_var.lnge (use of undeclared name), tests/fixtures/invalid/empty.lnge (empty file). Add integration tests in tests/integration/error_tests.rs verifying error output contains expected file:line:column and message substrings

**Checkpoint**: All error paths produce clear, located error messages. SC-004 and SC-005 satisfied.

---

## Phase 7: Polish & Cross-Cutting Concerns

**Purpose**: Performance validation, test coverage, edge cases, optimization passes

- [x] T045 [P] Implement optimization level mapping: O0=no passes, O1=module.run_passes("default<O1>"), O2=module.run_passes("default<O2>"), O3=module.run_passes("default<O3>") using PassBuilderOptions. Create TargetMachine with get_host_cpu_name() and get_host_cpu_features() instead of "generic" in src/codegen/mod.rs
- [x] T046 [P] Implement --emit llvm-ir option: print LLVM IR to stdout via module.print_to_string() when --emit llvm-ir is specified in src/codegen/mod.rs and src/main.rs
- [x] T047 [P] Implement --emit obj option: emit object file without linking when --emit obj is specified in src/main.rs
- [x] T048 Add Fibonacci(40) performance benchmark: (1) compile fib.lnge with -O2, verify result, measure compile time <2s; (2) compile equivalent C with gcc -O2, compare execution time (assert within 1.2x); (3) compile equivalent Go with go build, compare execution time (assert LNGE is faster). Create benchmark scripts and reference programs in tests/benchmarks/ and tests/integration/compile_run.rs
- [x] T049 [P] Add unit tests for Lexer: test token stream for representative source strings covering all token types in tests/unit/lexer_tests.rs
- [x] T050 [P] Add unit tests for Parser: test AST construction for function definitions, expressions with precedence, control flow, variable declarations in tests/unit/parser_tests.rs
- [x] T051 [P] Add unit tests for semantic analysis: test name resolution errors, type checking errors, immutability enforcement in tests/unit/semantic_tests.rs
- [x] T052 Run cargo clippy and fix all warnings, run cargo fmt to ensure consistent formatting
- [x] T053 Validate quickstart.md: follow the quickstart guide end-to-end, verify all commands work and produce expected output

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies - start immediately
- **Foundational (Phase 2)**: Depends on Setup (Phase 1) completion - BLOCKS all user stories
- **US4 - Functions (Phase 3)**: Depends on Foundational (Phase 2) - first vertical slice through the pipeline
- **US1 - Scalar Arithmetic (Phase 4)**: Depends on US4 (Phase 3) - extends expression codegen
- **US3 - Variables & Control Flow (Phase 5)**: Depends on US1 (Phase 4) - extends parser, semantic, codegen
- **US2 - Error Messages (Phase 6)**: Depends on US4 (Phase 3) minimum - can start after basic pipeline exists
- **Polish (Phase 7)**: Depends on US3 (Phase 5) and US2 (Phase 6) completion

### User Story Dependencies

- **US4 (P1)**: Foundation → Functions. First story, no other story dependencies. Delivers the minimal compilable program.
- **US1 (P1)**: US4 → Arithmetic. Extends expression handling built in US4. Cannot test arithmetic without function bodies from US4.
- **US3 (P1)**: US1 → Variables & Control Flow. Extends the parser and codegen built in US4+US1. Fibonacci requires both arithmetic (US1) and functions (US4).
- **US2 (P2)**: US4 → Error Messages. Can begin once basic pipeline exists. Does not depend on US1 or US3 for error infrastructure, but full error coverage benefits from all features being present.

### Within Each User Story

- Lexer extensions before parser extensions
- Parser before semantic analysis
- Semantic analysis before codegen
- Codegen before integration tests
- All pipeline stages for a feature before the integration test for that feature

### Parallel Opportunities

- Phase 2: All foundational tasks (T005-T012) can run in parallel — different files, no dependencies
- Phase 4: T024 and T025 (codegen operators) can run in parallel with T026 (type checking) — different files
- Phase 7: T045, T046, T047 (emit options) can run in parallel; T049, T050, T051 (unit tests) can run in parallel

---

## Parallel Examples

### Phase 2 (Foundational) — All parallel

```
T005: src/common/span.rs
T006: src/common/types.rs
T007: src/common/mod.rs        (after T005, T006)
T008: src/lexer/token.rs
T009: src/parser/ast.rs
T010: src/parser/precedence.rs
T011: src/diagnostics/mod.rs
T012: src/codegen/types.rs
```

### Phase 4 (US1 - Scalar Arithmetic) — Partial parallel

```
Parallel group: T024 + T025 (codegen) || T026 (typechecker) || T027 (parser types)
Sequential:     T028 (integration tests, after all above)
```

### Phase 7 (Polish) — Partial parallel

```
Parallel group: T045 + T046 + T047 (emit options)
Parallel group: T049 + T050 + T051 (unit tests)
Sequential:     T048, T052, T053
```

---

## Implementation Strategy

### MVP First (User Story 4 Only)

1. Complete Phase 1: Setup
2. Complete Phase 2: Foundational
3. Complete Phase 3: US4 - Functions
4. **STOP and VALIDATE**: `fn main() -> i32 { return 42; }` compiles and runs

### Incremental Delivery

1. Setup + Foundational → Types and infrastructure ready
2. Add US4 (Functions) → First compilable program (MVP!)
3. Add US1 (Arithmetic) → Expressions work: `return 2 + 3 * 4;`
4. Add US3 (Variables + Control Flow) → Fibonacci works!
5. Add US2 (Error Messages) → Developer-friendly error experience
6. Polish → Optimization, testing, cleanup

---

## Notes

- [P] tasks = different files, no dependencies between them
- [Story] label maps task to user story for traceability
- The compiler pipeline (lexer → parser → semantic → codegen) means stories build on each other more than typical CRUD apps
- US4 is placed before US1 because functions are needed before arithmetic is useful (need fn main to test anything)
- Commit after each task or logical group
- Stop at any checkpoint to validate the compiler increment
