# Research: Operators, Casting & Comments (Phase 3)

**Date**: 2026-04-01
**Feature**: 003-operators-casting

## R1: Compound Assignment Desugaring

**Decision**: Desugar compound assignment in the parser. `x += y` becomes `Stmt::Assign { target: x, value: BinOp(Add, x, y) }`.

**Rationale**: No new AST node needed. The parser reads the compound operator, extracts the target expression and the binary operation, and synthesizes a standard `Assign` statement. The semantic analysis and codegen see a normal assignment with a binary operation — zero changes needed downstream.

**Key detail**: For complex targets (`arr[i] += 1`, `v.x += 1.0`, `*p += 1`), the parser must evaluate the target expression only once. Since Ny has no side effects in index/field expressions (no function calls in index position), evaluating twice is semantically identical. Safe to desugar naively.

## R2: Bitwise Operator Precedence

**Decision**: Follow C precedence for bitwise operators.

| Operator | Binding Power (L, R) | Category |
|----------|----------------------|----------|
| `\|` | (1, 2) | Bitwise OR (same level as logical OR) |
| `^` | (3, 4) | Bitwise XOR |
| `&` (infix) | (5, 6) | Bitwise AND — **NOTE**: conflicts with comparison at (5,6). Place bitwise below comparison. |

**Revised precedence table** (full):

| Level | Operators | BP (L, R) |
|-------|-----------|-----------|
| Lowest | `\|\|` (logical OR) | (1, 2) |
| | `&&` (logical AND) | (3, 4) |
| | `== != < > <= >=` | (5, 6) |
| | `\|` (bitwise OR) | (7, 8) |
| | `^` (bitwise XOR) | (9, 10) |
| | `&` (bitwise AND) | (11, 12) |
| | `<< >>` (shifts) | (13, 14) |
| | `+ -` | (15, 16) |
| | `* / %` | (17, 18) |
| Highest | Unary `- ! ~ &(addr) *(deref)` | (_, 19) |

**Note**: This differs slightly from C where bitwise AND is above equality. Following Rust's precedence instead, which is more intuitive — comparisons always bind tighter than bitwise ops.

## R3: Disambiguating `&` and `|`

**Decision**: Context-based disambiguation via Pratt parsing.

- `&` in prefix position (returned by `parse_prefix`) → `Expr::AddrOf` (already implemented)
- `&` in infix position (returned by `infix_bp`) → `BinOp::BitAnd`
- `|` in infix position → `BinOp::BitOr` (new — currently errors with "did you mean ||?")
- `|` is never valid in prefix position

The existing Pratt parser naturally handles this: `parse_prefix` handles prefix operators, `infix_bp` handles infix. The `&` token is already handled as prefix `AddrOf` in `parse_prefix`. Adding it to `infix_bp` makes it available as infix `BitAnd`.

For `|`: Remove the error in the lexer ("did you mean ||?") and emit `TokenKind::Pipe` instead. `||` remains `TokenKind::Or`.

## R4: Type Casting LLVM Operations

**Decision**: Map cast operations to LLVM conversion intrinsics.

| Source → Target | LLVM Operation |
|----------------|----------------|
| Signed int → wider signed int | `build_int_s_extend` |
| Unsigned int → wider unsigned int | `build_int_z_extend` |
| Any int → narrower int | `build_int_truncate` |
| Signed int → float | `build_signed_int_to_float` |
| Unsigned int → float | `build_unsigned_int_to_float` |
| Float → signed int | `build_float_to_signed_int` |
| Float → unsigned int | `build_float_to_unsigned_int` |
| f32 → f64 | `build_float_ext` |
| f64 → f32 | `build_float_trunc` |
| bool → int | `build_int_z_extend` (i1 → target width) |
| Same type | No-op (return value unchanged) |

**`as` precedence**: Treated as a postfix operator with binding power 20 (highest, above unary). Parsed in the postfix section of `parse_expr`, similar to `.` and `[]`.

## R5: Block Comments

**Decision**: Implement nested block comments in `skip_whitespace_and_comments()` with a depth counter.

**Pattern**:
```
if peek == '/' && peek_next == '*':
    depth = 1
    advance twice
    while depth > 0:
        if peek == '/' && peek_next == '*': depth += 1; advance twice
        elif peek == '*' && peek_next == '/': depth -= 1; advance twice
        elif peek == None: error "unterminated block comment"
        else: advance
```

## R6: New Tokens Required

| Token | Lexeme | Purpose |
|-------|--------|---------|
| `Pipe` | `\|` | Bitwise OR (infix) |
| `Caret` | `^` | Bitwise XOR |
| `Tilde` | `~` | Bitwise NOT (prefix) |
| `LtLt` | `<<` | Left shift |
| `GtGt` | `>>` | Right shift |
| `As` | `as` | Type cast keyword |
| `PlusAssign` | `+=` | Compound add |
| `MinusAssign` | `-=` | Compound subtract |
| `StarAssign` | `*=` | Compound multiply |
| `SlashAssign` | `/=` | Compound divide |
| `PercentAssign` | `%=` | Compound modulo |
| `AmpAssign` | `&=` | Compound bitwise AND |
| `PipeAssign` | `\|=` | Compound bitwise OR |
| `CaretAssign` | `^=` | Compound bitwise XOR |
| `LtLtAssign` | `<<=` | Compound left shift |
| `GtGtAssign` | `>>=` | Compound right shift |
