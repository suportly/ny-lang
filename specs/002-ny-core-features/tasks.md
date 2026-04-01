# Tasks: Ny Lang Core Features (Phase 2)

**Input**: Design documents from `/specs/002-ny-core-features/`
**Prerequisites**: plan.md, spec.md, research.md, data-model.md, contracts/syntax-contract.md

**Organization**: Tasks are grouped by user story. All stories extend the same compiler pipeline files, so the order matters: foundational type system changes first, then each story builds on the previous.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (US1-US7)

---

## Phase 1: Setup

**Purpose**: New tokens, AST nodes, and type system extensions needed by ALL user stories

- [x] T001 Add new token variants to src/lexer/token.rs: Struct, For, In, Break, Continue, DotDot, DotDotEq, Dot, Ampersand, LBracket, RBracket, StringLit(String), ColonAssign, ColonTildeAssign
- [x] T002 Add new AST nodes to src/parser/ast.rs: Item::StructDef, Stmt::ForRange/Break/Continue, Expr::ArrayLit/Index/FieldAccess/StructInit/AddrOf/Deref/MethodCall, TypeAnnotation::Array/Pointer variants
- [x] T003 [P] Extend NyType enum in src/common/types.rs: add Array { elem, size }, Struct { name, fields }, Pointer(Box<NyType>), Str variants with Display impl and helper methods (is_array, is_struct, is_pointer, elem_type, field_type)
- [x] T004 [P] Extend LLVM type mapping in src/codegen/types.rs: add ny_to_llvm cases for NyType::Array (elem.array_type(n)), NyType::Struct (context.opaque_struct_type + set_body), NyType::Pointer (context.ptr_type), NyType::Str ({ptr, i64} struct)

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Lexer extensions for all new tokens — MUST complete before any user story parsing

- [x] T005 Extend Lexer in src/lexer/mod.rs: scan `.` (Dot), `..` (DotDot), `..=` (DotDotEq), `&` (Ampersand), `[` (LBracket), `]` (RBracket), `:=` (ColonAssign), `:~=` (ColonTildeAssign) tokens. Update Colon scanning to check for `:=` before falling through to Colon, and ColonTilde to check for `:~=` before ColonTilde
- [x] T006 Extend Lexer in src/lexer/mod.rs: scan string literals with double quotes, handling escape sequences (\n, \t, \\, \"), producing StringLit(String) token. Handle unterminated string error
- [x] T007 Extend Lexer keyword recognition in src/lexer/mod.rs: add `struct`, `for`, `in`, `break`, `continue` to the keyword match in read_ident_or_keyword()
- [x] T008 Add loop_stack field (Vec<LoopFrame> with break_bb/continue_bb) to CodeGen struct in src/codegen/mod.rs. Define LoopFrame struct. Update existing While codegen to push/pop LoopFrame with continue_bb=cond_bb, break_bb=exit_bb

**Checkpoint**: Lexer produces all new tokens. Codegen has loop stack ready.

---

## Phase 3: User Story 5 — For Loops with Ranges (Priority: P2)

**Goal**: `for i in 0..n { body }` and `for i in 0..=n { body }` range loops.

**Independent Test**: `for i in 0..10 { sum = sum + i; }` produces sum=45.

**Why P2 before P1**: For loops are needed by array iteration (US1), so implementing them first unblocks arrays.

### Implementation for User Story 5

- [x] T009 [US5] Implement parse_for_range() in src/parser/mod.rs: parse `for ident in expr..expr { block }` and `for ident in expr..=expr { block }` producing Stmt::ForRange with var, start, end, inclusive flag, body
- [x] T010 [US5] Add ForRange type checking in src/semantic/typechecker.rs: validate start and end are same integer type, declare loop variable as immutable in body scope, check body
- [x] T011 [US5] Add ForRange name resolution in src/semantic/resolver.rs: declare loop variable in new scope for body, resolve start/end/body expressions
- [x] T012 [US5] Implement ForRange codegen in src/codegen/mod.rs: create 4 basic blocks (for_cond, for_body, for_inc, for_exit), alloca loop var, store start, condition check (SLT for exclusive, SLE for inclusive), body compilation, increment, push/pop LoopFrame
- [x] T013 [US5] Create test fixtures tests/fixtures/valid/for_range.ny (sum 0..10=45, sum 0..=10=55, empty range, nested for loops) and integration test in tests/compile_run.rs

**Checkpoint**: For loops work. `for i in 0..10` iterates correctly.

---

## Phase 4: User Story 7 — Break and Continue (Priority: P2)

**Goal**: `break` exits innermost loop, `continue` skips to next iteration.

**Independent Test**: Search array for value with break, sum even numbers with continue.

### Implementation for User Story 7

- [x] T014 [US7] Implement parse_break() and parse_continue() in src/parser/mod.rs: parse `break;` and `continue;` statements producing Stmt::Break and Stmt::Continue
- [x] T015 [US7] Add break/continue validation in src/semantic/resolver.rs: track loop depth, report error if break/continue used outside loop
- [x] T016 [US7] Implement break/continue codegen in src/codegen/mod.rs: break builds unconditional branch to loop_stack.last().break_bb, continue branches to loop_stack.last().continue_bb. Check loop_stack not empty (redundant with semantic check but defensive)
- [x] T017 [US7] Create test fixtures tests/fixtures/valid/break_continue.ny (break from while, break from for, continue in for, nested break) and tests/fixtures/invalid/break_outside_loop.ny. Add integration tests in tests/compile_run.rs and tests/error_tests.rs

**Checkpoint**: Break and continue work in both while and for loops.

---

## Phase 5: User Story 1 — Arrays and Indexed Data (Priority: P1)

**Goal**: Fixed-size arrays `[N]T`, array literals, indexing with bounds checking, arrays in functions.

**Independent Test**: Declare `[5]i32`, sum elements in for loop, return sum as exit code.

### Implementation for User Story 1

- [x] T018 [US1] Implement array type annotation parsing in src/parser/mod.rs: parse `[N]T` as TypeAnnotation::Array producing size and element type. Handle nested arrays `[M][N]T`
- [x] T019 [US1] Implement array literal parsing in src/parser/mod.rs: parse `[expr1, expr2, ..., exprN]` as Expr::ArrayLit. Handle empty brackets as potential ambiguity with index expr
- [x] T020 [US1] Implement index expression parsing in src/parser/mod.rs: parse `expr[expr]` as Expr::Index. Handle as postfix operator after primary expression (ident, call result, etc.)
- [x] T021 [US1] Add array type checking in src/semantic/typechecker.rs: validate ArrayLit element count matches declared size, all elements same type, Index expression requires array type on left and integer type for index
- [x] T022 [US1] Add array name resolution in src/semantic/resolver.rs: resolve TypeAnnotation::Array to NyType::Array, resolve array literal elements, resolve index expressions
- [x] T023 [US1] Implement array codegen in src/codegen/mod.rs: ArrayLit → alloca array type + store each element via GEP. Index read → GEP with bounds check + load. Index write → GEP with bounds check + store. Bounds check: compare index ULT length, branch to abort on failure
- [x] T024 [US1] Implement array function passing codegen in src/codegen/mod.rs: arrays passed by value (load entire array, pass as ArrayValue), arrays in return values
- [x] T025 [US1] Create test fixtures tests/fixtures/valid/arrays.ny (array literal, indexing, for-loop sum, nested arrays, array as function param) and tests/fixtures/invalid/array_bounds.ny (out-of-bounds access). Add integration tests

**Checkpoint**: Arrays work. Dot product and sum-of-array programs compile and run correctly.

---

## Phase 6: User Story 2 — Structs with Methods (Priority: P1)

**Goal**: Struct definitions, field access, methods with self parameter, value semantics.

**Independent Test**: Define Vec2, create instance, access fields, call method, verify result.

### Implementation for User Story 2

- [x] T026 [US2] Implement struct definition parsing in src/parser/mod.rs: parse `struct Name { field: Type, ... }` as Item::StructDef at top level
- [x] T027 [US2] Implement struct instantiation parsing in src/parser/mod.rs: parse `Name { field: expr, ... }` as Expr::StructInit
- [x] T028 [US2] Implement field access parsing in src/parser/mod.rs: parse `expr.field` as Expr::FieldAccess (postfix, same precedence as index)
- [x] T029 [US2] Implement method call parsing in src/parser/mod.rs: parse `expr.method(args)` as Expr::MethodCall (disambiguate from field access by checking for `(` after field name)
- [x] T030 [US2] Add struct type registration in src/semantic/resolver.rs: first pass registers all struct definitions with field names/types. Validate no recursive structs by value, no duplicate field names
- [x] T031 [US2] Add struct type checking in src/semantic/typechecker.rs: validate StructInit field names/types match definition, FieldAccess on struct type returns field type, MethodCall resolves to function with matching self parameter
- [x] T032 [US2] Implement struct codegen in src/codegen/mod.rs: register LLVM named struct types in first pass. StructInit → alloca + store each field via build_struct_gep. FieldAccess → build_struct_gep + load. Method call → call function with self value as first arg
- [x] T033 [US2] Create test fixtures tests/fixtures/valid/structs.ny (struct def, instantiation, field access, method call, struct as function param, nested structs) and tests/fixtures/invalid/struct_recursive.ny. Add integration tests

**Checkpoint**: Structs work. Vec2 with dot product method compiles and runs.

---

## Phase 7: User Story 3 — Pointers (Priority: P1)

**Goal**: Pointer types `*T`, address-of `&`, dereference `*`, auto-deref for struct fields.

**Independent Test**: Pass array pointer to function, modify through pointer, verify caller sees change.

### Implementation for User Story 3

- [x] T034 [US3] Implement pointer type annotation parsing in src/parser/mod.rs: parse `*T` as TypeAnnotation::Pointer
- [x] T035 [US3] Implement address-of and dereference expression parsing in src/parser/mod.rs: parse `&expr` as Expr::AddrOf (prefix), parse `*expr` as Expr::Deref (prefix, disambiguate from multiply by context — prefix position vs infix)
- [x] T036 [US3] Add pointer type checking in src/semantic/typechecker.rs: AddrOf produces Pointer(T) where T is operand type, Deref of Pointer(T) produces T, auto-deref for FieldAccess on pointer-to-struct
- [x] T037 [US3] Add pointer resolution in src/semantic/resolver.rs: resolve TypeAnnotation::Pointer to NyType::Pointer, resolve AddrOf/Deref expressions
- [x] T038 [US3] Implement pointer codegen in src/codegen/mod.rs: AddrOf → return the alloca pointer directly (no extra alloca). Deref read → build_load with pointee type. Deref write (*p = v) → build_store to pointer. Auto-deref for FieldAccess → build_struct_gep on pointer directly
- [x] T039 [US3] Create test fixtures tests/fixtures/valid/pointers.ny (address-of, deref read/write, pointer to array, pointer to struct, auto-deref field access, swap function) and integration tests

**Checkpoint**: Pointers work. Can pass arrays/structs by pointer and mutate through them.

---

## Phase 8: User Story 4 — Strings and Print (Priority: P1)

**Goal**: `str` type, string literals, `print(value)` and `println(value)` for all types.

**Independent Test**: `println("Hello, Ny!")` produces correct stdout output.

### Implementation for User Story 4

- [x] T040 [US4] Add str type handling in src/semantic/resolver.rs and src/semantic/typechecker.rs: recognize "str" as NyType::Str, StringLit expressions produce NyType::Str
- [x] T041 [US4] Implement string literal codegen in src/codegen/mod.rs: StringLit → create global constant via module.add_global with const_string, return {ptr, len} struct value. Handle escape sequences during lexing (already in T006) so codegen receives the actual bytes
- [x] T042 [US4] Declare printf and write external functions in src/codegen/mod.rs: add printf (variadic, i32 return) and write (fd, buf, len) to module on initialization
- [x] T043 [US4] Implement print/println codegen in src/codegen/mod.rs: recognize print/println as built-in function calls. Dispatch by argument type: integers → printf("%d"), i64 → printf("%ld"), f64 → printf("%f"), bool → conditional "true"/"false", str → write(1, ptr, len), structs → printf each field with "Name { f1: v1, f2: v2 }" format. println appends "\n"
- [x] T044 [US4] Handle print/println in semantic analysis in src/semantic/resolver.rs and src/semantic/typechecker.rs: recognize print/println as built-in names that accept any single argument (skip normal function resolution for these)
- [x] T045 [US4] Create test fixtures tests/fixtures/valid/hello_print.ny (println string, int, float, bool, struct) and integration tests that capture stdout and verify content

**Checkpoint**: Print works. Programs can output text and numbers to stdout.

---

## Phase 9: User Story 6 — Type Inference (Priority: P2)

**Goal**: `x := 5;` (immutable inferred) and `x :~= 5;` (mutable inferred).

**Independent Test**: `x := 5; y := 3.14; println(x); println(y);` compiles and prints correct values.

### Implementation for User Story 6

- [x] T046 [US6] Implement `:=` and `:~=` parsing in src/parser/mod.rs: when scanning var-decl-or-assign, detect ColonAssign → produce VarDecl with mutability=Immutable and ty=None. Detect ColonTildeAssign → produce VarDecl with mutability=Mutable and ty=None. The existing type checker already handles ty=None by inferring from init expression
- [x] T047 [US6] Create test fixtures tests/fixtures/valid/inference.ny (infer i32, f64, bool, struct types; mutable inferred with :~=; immutability error on := variable reassignment) and integration tests

**Checkpoint**: Type inference works. `:=` and `:~=` reduce verbosity.

---

## Phase 10: Polish & Cross-Cutting Concerns

**Purpose**: Integration tests, benchmarks, code quality

- [x] T048 [P] Create tests/fixtures/valid/matrix.ny: 3x3 matrix multiplication using nested for loops on `[3][3]f64` arrays, with struct-based Matrix type and methods. Verify correct result via println
- [x] T049 [P] Create tests/fixtures/valid/dot_product.ny: dot product of two [100]f64 arrays, verify correct result
- [x] T050 Run cargo fmt and cargo clippy --fix, resolve all warnings
- [x] T051 Update examples/ directory: rename .lnge fixtures to .ny, add examples/arrays.ny, examples/structs.ny, examples/matrix.ny demonstrating Phase 2 features
- [x] T052 Update README.md: add Phase 2 features to feature list, add array/struct/pointer syntax examples, update examples section

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies — new tokens, AST, types
- **Foundational (Phase 2)**: Depends on Phase 1 — lexer must scan new tokens before parsing
- **US5 For Loops (Phase 3)**: Depends on Phase 2 — needed before arrays (for iteration)
- **US7 Break/Continue (Phase 4)**: Depends on US5 (for loops must exist first)
- **US1 Arrays (Phase 5)**: Depends on US5 (for loops needed for array iteration tests)
- **US2 Structs (Phase 6)**: Depends on Phase 2 only — independent of arrays
- **US3 Pointers (Phase 7)**: Depends on US1 + US2 (pointer-to-array and pointer-to-struct)
- **US4 Strings/Print (Phase 8)**: Depends on Phase 2 only — can start after foundational
- **US6 Inference (Phase 9)**: Depends on Phase 2 only — minimal changes
- **Polish (Phase 10)**: Depends on all user stories

### Parallel Opportunities

- Phase 1: T003 and T004 can run in parallel (different files)
- After Phase 2: US2 (Structs), US4 (Strings/Print), and US6 (Inference) are independent of each other
- Phase 10: T048 and T049 can run in parallel

### Within Each User Story

- Parser changes before semantic analysis
- Semantic analysis before codegen
- Codegen before test fixtures

---

## Implementation Strategy

### MVP First (For Loops + Arrays)

1. Phase 1: Setup (types, tokens, AST)
2. Phase 2: Foundational (lexer)
3. Phase 3: US5 For Loops
4. Phase 5: US1 Arrays
5. **STOP and VALIDATE**: sum-of-array and dot-product programs work

### Incremental Delivery

1. Setup + Foundational → New tokens and types ready
2. US5 (For Loops) → `for i in 0..10` works
3. US7 (Break/Continue) → Loop control flow complete
4. US1 (Arrays) → `[N]T` with indexing and for-loop iteration
5. US2 (Structs) → Custom types with methods
6. US3 (Pointers) → Efficient data passing
7. US4 (Strings/Print) → Observable output
8. US6 (Inference) → Reduced verbosity
9. Polish → Matrix multiplication, benchmarks, docs
