# Tasks: Strings, Enums & Tuples (Phase 4)

**Input**: Design documents from `/specs/004-strings-enums-tuples/`
**Prerequisites**: plan.md, spec.md, research.md

**Organization**: Tasks grouped by user story. Strings and enums are independent of each other. Tuples depend on neither but benefit from match (for Result-like patterns).

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: US1=Strings, US2=Enums, US3=Match, US4=Tuples

---

## Phase 1: Setup (New Types, Tokens, AST)

**Purpose**: Type system extensions, new tokens, and AST nodes shared across all stories

- [x] T001 [P] Add NyType::Enum { name: String, variants: Vec<String> } and NyType::Tuple(Vec<NyType>) to src/common/types.rs with Display impl, helper methods (is_enum, is_tuple, variant_index)
- [x] T002 [P] Add new tokens to src/lexer/token.rs: Enum, Match, FatArrow (=>), Underscore (_) keywords
- [x] T003 Add new AST nodes to src/parser/ast.rs: Item::EnumDef { name, variants, span }, Expr::Match { subject, arms, span }, MatchArm { pattern: Pattern, body: Expr }, Pattern enum (EnumVariant, IntLit, Wildcard), Expr::TupleLit { elements, span }, Expr::TupleIndex { object, index: usize, span }, Stmt::TupleDestructure { names, mutability, init, span }, TypeAnnotation::Tuple { elements, span }
- [x] T004 [P] Extend LLVM type mapping in src/codegen/types.rs: NyType::Enum → context.i32_type(), NyType::Tuple → context.struct_type(&[field_types], false) (anonymous struct)

---

## Phase 2: Foundational (Lexer)

**Purpose**: Scan all new tokens — MUST complete before parsing

- [x] T005 Extend lexer in src/lexer/mod.rs: add `enum`, `match` keywords to read_ident_or_keyword(). Scan `=>` as FatArrow (in `=` handler: check next char is `>`, else Assign). Scan `_` as Underscore when standalone (not part of identifier — `_` at start of ident is already handled, so emit Underscore only when `_` is not followed by alphanumeric)
- [x] T006 Declare extern functions in codegen initialization in src/codegen/mod.rs: add `malloc`, `memcpy`, `memcmp`, `free` to the libc function declarations alongside existing printf/write/abort

**Checkpoint**: Lexer scans all new tokens. Runtime functions declared.

---

## Phase 3: User Story 1 — String Operations (Priority: P1)

**Goal**: `str.len()`, `str + str` (concat), `str == str` / `str != str` (compare), `str.substr(start, end)`.

**Independent Test**: Concat two strings, check length, compare, print result.

### Implementation for User Story 1

- [x] T007 [US1] Implement string comparison in src/codegen/mod.rs: in compile_binop(), when both operands are NyType::Str, emit length comparison + memcmp call for == and !=. Extract ptr and len fields from the {ptr, len} struct using build_extract_value
- [x] T008 [US1] Implement string comparison type checking in src/semantic/typechecker.rs: allow BinOp::Eq and BinOp::Ne for NyType::Str operands (currently only numeric types pass comparison checks)
- [x] T009 [US1] Implement string concatenation in src/codegen/mod.rs: in compile_binop(), when both operands are NyType::Str and op is Add, emit malloc(a.len + b.len), memcpy(ptr, a.ptr, a.len), memcpy(ptr+a.len, b.ptr, b.len), return {new_ptr, a.len + b.len}
- [x] T010 [US1] Implement string concat type checking in src/semantic/typechecker.rs: allow BinOp::Add for two NyType::Str operands, return NyType::Str
- [x] T011 [US1] Implement str.len() method in src/codegen/mod.rs: in method call dispatch, when receiver is NyType::Str and method is "len", extract field 1 (length) from the {ptr, len} struct
- [x] T012 [US1] Implement str.substr(start, end) method in src/codegen/mod.rs: when receiver is Str and method is "substr", compute new_ptr = ptr + start, new_len = min(end, len) - start, return {new_ptr, new_len}
- [x] T013 [US1] Add str method type checking in src/semantic/typechecker.rs: recognize "len" returning NyType::I64, "substr" taking (i64, i64) returning NyType::Str on Str receiver
- [x] T014 [US1] Create test fixture tests/fixtures/valid/string_ops.ny (concat, len, compare, substr) and add integration test in tests/compile_run.rs

**Checkpoint**: String operations work. Concat allocates, len/substr/compare work.

---

## Phase 4: User Story 2 — Enums (Priority: P1)

**Goal**: `enum Color { Red, Green, Blue }`, `Color::Red`, comparison, printing.

**Independent Test**: Define enum, compare variants, print variant name.

### Implementation for User Story 2

- [x] T015 [US2] Implement enum definition parsing in src/parser/mod.rs: parse `enum Name { Variant1, Variant2, ... }` as Item::EnumDef at top level (alongside fn and struct)
- [x] T016 [US2] Implement enum variant expression parsing in src/parser/mod.rs: parse `Ident::Ident` (e.g., `Color::Red`) as Expr::EnumVariant when the first ident is a known enum name. Handle in parse_prefix after Ident, when followed by ColonColon
- [x] T017 [US2] Add enum type registration in src/semantic/resolver.rs: first pass registers all enum definitions in a HashMap<String, Vec<String>>. Resolve variant expressions
- [x] T018 [US2] Add enum type checking in src/semantic/typechecker.rs: EnumVariant produces NyType::Enum { name, variants }. Eq/Ne comparison valid on same-enum types
- [x] T019 [US2] Implement enum codegen in src/codegen/mod.rs: register enums in first pass (store variant→index mapping). EnumVariant compiles to i32 constant (variant index). Eq/Ne on enums uses integer comparison
- [x] T020 [US2] Implement enum printing in src/codegen/mod.rs: for println on enum type, use switch on discriminant to select variant name string, then print it
- [x] T021 [US2] Create test fixture tests/fixtures/valid/enums.ny (enum def, variant access, comparison, printing, passing to functions) and add integration test

**Checkpoint**: Enums work. Variants compare correctly, print variant names.

---

## Phase 5: User Story 3 — Match Expressions (Priority: P1)

**Goal**: `match expr { Pattern => body, ... }` with exhaustiveness checking.

**Independent Test**: Match on enum variant, return different values per arm.

### Implementation for User Story 3

- [x] T022 [US3] Implement match expression parsing in src/parser/mod.rs: parse `match expr { Pattern => expr, Pattern => { block }, ... }` as Expr::Match. Parse patterns: EnumName::Variant, integer literals, _ (wildcard). Arms separated by commas
- [x] T023 [US3] Add match type checking in src/semantic/typechecker.rs: validate subject type matches patterns, all arms return same type, exhaustiveness check for enums (all variants covered or _ present)
- [x] T024 [US3] Add match resolution in src/semantic/resolver.rs: resolve match subject and all arm bodies
- [x] T025 [US3] Implement match codegen in src/codegen/mod.rs: compile subject, use build_switch for enum/integer subjects with basic blocks per arm. Build phi node at merge block to collect arm results. Default arm maps to _ pattern
- [x] T026 [US3] Create test fixture tests/fixtures/valid/match_expr.ny (match on enum, match on integer, match as expression, block bodies, wildcard, exhaustiveness) and tests/fixtures/invalid/match_nonexhaustive.ny. Add integration tests

**Checkpoint**: Match expressions work. Exhaustiveness enforced.

---

## Phase 6: User Story 4 — Tuples (Priority: P1)

**Goal**: `(T1, T2)` types, `(v1, v2)` literals, `.0`/`.1` access, multi-return, destructuring.

**Independent Test**: Function returns `(i32, bool)`, destructured at call site.

### Implementation for User Story 4

- [x] T027 [US4] Implement tuple type annotation parsing in src/parser/mod.rs: parse `(T1, T2, ...)` as TypeAnnotation::Tuple. Disambiguate from parenthesized type by requiring comma
- [x] T028 [US4] Implement tuple literal parsing in src/parser/mod.rs: parse `(expr1, expr2, ...)` as Expr::TupleLit. Disambiguate from parenthesized expr by requiring comma after first element
- [x] T029 [US4] Implement tuple index parsing in src/parser/mod.rs: in postfix section of parse_expr, after `.`, if next token is IntLit, produce Expr::TupleIndex { object, index }
- [x] T030 [US4] Implement tuple destructuring parsing in src/parser/mod.rs: parse `(a, b) := expr;` and `(a, b) :~= expr;` as Stmt::TupleDestructure. Detect when statement starts with `(` followed by idents and commas
- [x] T031 [US4] Add tuple type checking in src/semantic/typechecker.rs: TupleLit produces NyType::Tuple(element_types). TupleIndex validates index < tuple length, returns element type. TupleDestructure validates init is tuple with matching element count
- [x] T032 [US4] Add tuple resolution in src/semantic/resolver.rs: resolve TupleLit elements, TupleIndex object, TupleDestructure init and declared names
- [x] T033 [US4] Implement tuple codegen in src/codegen/mod.rs: TupleLit → build anonymous struct with values. TupleIndex → build_extract_value(struct, index). TupleDestructure → compile init expr, extract_value for each element, alloca + store each
- [x] T034 [US4] Implement tuple printing in src/codegen/mod.rs: for println on tuple type, print "(", then each element separated by ", ", then ")"
- [x] T035 [US4] Create test fixture tests/fixtures/valid/tuples.ny (tuple literal, indexing .0/.1, multi-return function, destructuring, printing, nested tuples) and add integration tests

**Checkpoint**: Tuples work. Multi-return and destructuring work.

---

## Phase 7: Polish

- [x] T036 Run cargo fmt and cargo clippy, fix all warnings
- [x] T037 Verify all Phase 1-3 tests still pass (no regressions) — run full `cargo test`
- [x] T038 Update README.md with Phase 4 features and syntax examples

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies
- **Foundational (Phase 2)**: Depends on Phase 1
- **US1 Strings (Phase 3)**: Depends on Phase 2 — independent of US2/US3/US4
- **US2 Enums (Phase 4)**: Depends on Phase 2 — independent of US1/US4
- **US3 Match (Phase 5)**: Depends on US2 (needs enum types for patterns)
- **US4 Tuples (Phase 6)**: Depends on Phase 2 — independent of US1/US2/US3
- **Polish (Phase 7)**: After all stories

### Parallel Opportunities

- Phase 1: T001, T002, T004 can run in parallel (different files)
- After Phase 2: US1 (Strings) and US2 (Enums) and US4 (Tuples) are independent
- US3 (Match) requires US2 (Enums) to be done first

---

## Implementation Strategy

### MVP: Enums + Match

1. Phase 1 + Phase 2 → types and tokens ready
2. US2 (Enums) → `enum Color { Red, Green, Blue }` works
3. US3 (Match) → `match c { Color::Red => 1, ... }` works
4. **STOP and VALIDATE**

### Full Delivery

5. US1 (Strings) → concat, len, compare, substr
6. US4 (Tuples) → multi-return and destructuring
7. Polish → fmt, clippy, README
