# Feature Specification: Tagged Unions & Loop (Phase 6)

**Feature Branch**: `004-strings-enums-tuples`
**Created**: 2026-04-01
**Status**: Implemented

## Summary

Extend enums with data-carrying variants (tagged unions), add variable binding in match patterns, and introduce the `loop` keyword for infinite loops.

## User Scenarios & Testing

### User Story 1 - Data-Carrying Enum Variants (Priority: P1)

A developer defines enums where variants carry typed payloads: `enum Shape { Circle(f64), Rect(f64, f64) }`. Variants are constructed with `Shape::Circle(3.14)` and destructured in match with `Shape::Circle(r) => ...`.

**Acceptance Scenarios**:

1. **Given** `enum Option { Some(i32), None }`, **When** compiled, **Then** the enum is recognized with two variants, one carrying an i32 payload.
2. **Given** `v := Option::Some(42);`, **When** compiled, **Then** `v` holds the `Some` variant with payload 42.
3. **Given** `match v { Option::Some(x) => x, Option::None => 0 }`, **When** `v` is `Some(42)`, **Then** `x` binds to 42 and the result is 42.

### User Story 2 - Loop Keyword (Priority: P1)

A developer uses `loop { ... }` for infinite loops, exiting with `break`.

**Acceptance Scenarios**:

1. **Given** `loop { if i >= 10 { break; } i += 1; }`, **When** run, **Then** the loop executes until `i >= 10`.
2. **Given** `continue` inside a `loop`, **When** executed, **Then** control jumps to the beginning of the loop body.

## Requirements

### Functional Requirements

- **FR-001**: Enum variants MAY carry typed payloads: `Variant(Type1, Type2, ...)`
- **FR-002**: Enum variant construction with payloads: `EnumName::Variant(arg1, arg2)`
- **FR-003**: Pattern matching with variable binding: `EnumName::Variant(name1, name2) => expr`
- **FR-004**: Bound variables are available in the match arm body
- **FR-005**: `loop { body }` creates an infinite loop (equivalent to `while true`)
- **FR-006**: `break` and `continue` work inside `loop` blocks

## Implementation

### AST Changes
- `EnumVariantDef { name, payload: Vec<TypeAnnotation>, span }` replaces `String` in enum defs
- `Expr::EnumVariant` gains `args: Vec<Expr>` field
- `Pattern::EnumVariant` gains `bindings: Vec<String>` field
- New `Stmt::Loop { body, span }`

### Type System
- `NyType::Enum.variants` changed from `Vec<String>` to `Vec<(String, Vec<NyType>)>`

### Tests
- `tests/fixtures/valid/loop_stmt.ny` — loop with break, sum 0..9 = 45
