# Tasks: Operators, Casting & Comments (Phase 3)

**Input**: Design documents from `/specs/003-operators-casting/`
**Prerequisites**: plan.md, spec.md, research.md

**Organization**: This phase adds operators and syntax to existing pipeline files. No new source files. Stories are mostly independent — bitwise and compound assignment share token scanning, but can be tested separately.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: US1=Compound assign, US2=Bitwise, US3=Casting, US4=Block comments

---

## Phase 1: Setup (New Tokens)

**Purpose**: Add all new token variants to the lexer — shared across all stories

- [x] T001 Add new token variants to src/lexer/token.rs: Pipe, Caret, Tilde, LtLt, GtGt, As, PlusAssign, MinusAssign, StarAssign, SlashAssign, PercentAssign, AmpAssign, PipeAssign, CaretAssign, LtLtAssign, GtGtAssign
- [x] T002 Add new AST nodes to src/parser/ast.rs: BinOp::BitAnd, BitOr, BitXor, Shl, Shr; UnaryOp::BitNot; Expr::Cast { expr: Box<Expr>, target_type: TypeAnnotation, span: Span }

---

## Phase 2: Foundational (Lexer + Precedence)

**Purpose**: Scan all new tokens and update operator precedence table

- [x] T003 Extend lexer in src/lexer/mod.rs: change `|` from error to Pipe token, `||` stays Or. Add `^` → Caret, `~` → Tilde. For `<`, check `<<` → LtLt, then `<<=` → LtLtAssign, else `<=` → Le, else Lt. Same for `>` → `>>` → GtGt, `>>=` → GtGtAssign, `>=` → Ge, `>` → Gt
- [x] T004 Extend lexer in src/lexer/mod.rs: for `+` check `+=` → PlusAssign else Plus. Same for `-` check `-=` (but also `->` → Arrow). For `*` check `*=` → StarAssign. For `/` check `/=` → SlashAssign (but also `//` comment and `/*` block comment). For `%` check `%=` → PercentAssign. For `&` check `&=` → AmpAssign, `&&` → And, else Ampersand. For `|` check `|=` → PipeAssign, `||` → Or, else Pipe. For `^` check `^=` → CaretAssign else Caret
- [x] T005 Add block comment scanning in src/lexer/mod.rs: in skip_whitespace_and_comments(), when `/*` detected, track nesting depth, consume until matching `*/`, error on unterminated
- [x] T006 Add `as` keyword to src/lexer/mod.rs read_ident_or_keyword() → TokenKind::As
- [x] T007 Update operator precedence in src/parser/precedence.rs: revise infix_bp() to new table — `||`(1,2), `&&`(3,4), comparisons(5,6), `|`(7,8), `^`(9,10), `&`(11,12), `<< >>`(13,14), `+ -`(15,16), `* / %`(17,18). Add prefix_bp for `~` → 19

**Checkpoint**: Lexer produces all new tokens. Precedence table updated.

---

## Phase 3: User Story 4 — Block Comments (Priority: P2)

**Goal**: `/* ... */` block comments with nesting support.

**Independent Test**: Compile a program with nested block comments, verify commented code is ignored.

### Implementation for User Story 4

- [x] T008 [US4] Create test fixture tests/fixtures/valid/block_comments.ny (program with inline, multi-line, and nested block comments) and add integration test in tests/compile_run.rs

**Checkpoint**: Block comments work. (Lexer scanning was done in T005.)

---

## Phase 4: User Story 1 — Compound Assignment (Priority: P1)

**Goal**: `+=`, `-=`, `*=`, `/=`, `%=` and compound bitwise `&=`, `|=`, `^=`, `<<=`, `>>=` on mutable variables, array indices, struct fields, and pointer dereferences.

**Independent Test**: `x :~ i32 = 0; for i in 0..10 { x += i; } return x;` → 45.

### Implementation for User Story 1

- [x] T009 [US1] Implement compound assignment parsing in src/parser/mod.rs: in parse_expr_or_assign_stmt() and parse_var_decl_or_assign(), when encountering PlusAssign/MinusAssign/StarAssign/SlashAssign/PercentAssign/AmpAssign/PipeAssign/CaretAssign/LtLtAssign/GtGtAssign after an expression, desugar to Stmt::Assign { target, value: BinOp(op, target_expr, rhs) }. Map each compound token to its BinOp variant
- [x] T010 [US1] Create test fixture tests/fixtures/valid/compound_assign.ny (all compound operators on variables, array indices, struct fields; immutability error test) and add integration tests in tests/compile_run.rs and tests/error_tests.rs

**Checkpoint**: Compound assignment works on all target types.

---

## Phase 5: User Story 2 — Bitwise Operators (Priority: P1)

**Goal**: `& | ^ << >>` binary operators and `~` unary NOT on integers.

**Independent Test**: `0xFF & 0x0F` → 15, `1 << 4` → 16, `~0` → -1.

### Implementation for User Story 2

- [x] T011 [US2] Add bitwise BinOp/UnaryOp handling to token_to_binop() in src/parser/mod.rs: map Ampersand→BitAnd (infix context), Pipe→BitOr, Caret→BitXor, LtLt→Shl, GtGt→Shr. Add Tilde→BitNot in parse_prefix()
- [x] T012 [US2] Add bitwise type checking in src/semantic/typechecker.rs: BitAnd/BitOr/BitXor/Shl/Shr require integer operands (not float, bool, str, struct), same-type check for AND/OR/XOR, shift RHS must be integer. BitNot requires integer operand
- [x] T013 [US2] Add bitwise codegen in src/codegen/mod.rs: in compile_binop() add BitAnd→build_and, BitOr→build_or, BitXor→build_xor, Shl→build_left_shift, Shr→build_right_shift (arithmetic for signed, logical for unsigned). In compile_unaryop() add BitNot→build_not
- [x] T014 [US2] Create test fixture tests/fixtures/valid/bitwise.ny (AND, OR, XOR, shifts, NOT, compound bitwise assign) and tests/fixtures/invalid/bitwise_float.ny (bitwise on float → error). Add integration tests

**Checkpoint**: All bitwise operators work on integer types.

---

## Phase 6: User Story 3 — Type Casting (Priority: P1)

**Goal**: `expr as T` for numeric type conversions.

**Independent Test**: `(3.14 as i32)` → 3, `(42 as i64)` → 42, `(true as i32)` → 1.

### Implementation for User Story 3

- [x] T015 [US3] Implement `as` parsing in src/parser/mod.rs: in parse_expr() postfix section (alongside `.` and `[]`), when As token is encountered, parse the target type annotation and produce Expr::Cast { expr, target_type, span }
- [x] T016 [US3] Add Cast to Expr::span() match in src/parser/ast.rs (already added in T002, verify)
- [x] T017 [US3] Add cast type checking in src/semantic/typechecker.rs: validate source and target are both scalar (integer, float, or bool). Reject casts involving structs, arrays, pointers, strings. Same-type cast is a no-op. Return the target type
- [x] T018 [US3] Add cast resolution in src/semantic/resolver.rs: resolve the Cast expression's inner expr and target_type TypeAnnotation
- [x] T019 [US3] Implement cast codegen in src/codegen/mod.rs: compile_cast() function dispatching on source/target type pairs — int→int (extend/truncate), int→float (si_to_fp/ui_to_fp), float→int (fp_to_si/fp_to_ui), float→float (ext/trunc), bool→int (z_extend), same-type (no-op)
- [x] T020 [US3] Create test fixture tests/fixtures/valid/casting.ny (int widening, int narrowing, float-to-int, int-to-float, float widening/narrowing, bool-to-int, chained casts) and tests/fixtures/invalid/cast_struct.ny (struct cast → error). Add integration tests

**Checkpoint**: Type casting works for all numeric conversions.

---

## Phase 7: Polish

- [x] T021 Run cargo fmt and cargo clippy, fix all warnings
- [x] T022 Verify all Phase 1 and Phase 2 tests still pass (no regressions) — run full `cargo test`
- [x] T023 Update examples/ with Phase 3 syntax where appropriate (e.g., use += in fibonacci loop)

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies — tokens and AST nodes
- **Foundational (Phase 2)**: Depends on Phase 1 — lexer must scan new tokens
- **US4 Block Comments (Phase 3)**: Depends on Phase 2 (T005 scanning) — test only
- **US1 Compound Assign (Phase 4)**: Depends on Phase 2 — needs compound tokens scanned
- **US2 Bitwise (Phase 5)**: Depends on Phase 2 — needs bitwise tokens and precedence
- **US3 Casting (Phase 6)**: Depends on Phase 2 — needs `as` keyword
- **Polish (Phase 7)**: After all stories

### Parallel Opportunities

- After Phase 2: US1, US2, US3, US4 are all independent of each other
- Within US2: T012 (type checking) and T013 (codegen) can run after T011 (parsing)

---

## Implementation Strategy

### MVP: Compound Assignment + Block Comments

1. Phase 1 + Phase 2 → tokens and lexer ready
2. US4 (Block Comments) → quick win
3. US1 (Compound Assignment) → `x += 1` works

### Full Delivery

4. US2 (Bitwise) → bit manipulation works
5. US3 (Casting) → type conversions work
6. Polish → clean up, verify no regressions
