# Feature Specification: Impl Blocks & Pub (Phase 7)

**Feature Branch**: `004-strings-enums-tuples`
**Created**: 2026-04-01
**Status**: Implemented

## Summary

Add `impl Type { ... }` blocks for defining methods on structs (replacing the `TypeName_method` convention), and the `pub` keyword for visibility annotations.

## User Scenarios & Testing

### User Story 1 - Impl Blocks (Priority: P1)

A developer groups methods for a struct inside an `impl` block. Methods with `self: Type` as the first parameter are callable with dot syntax.

**Acceptance Scenarios**:

1. **Given** `impl Point { fn magnitude_sq(self: Point) -> i32 { ... } }`, **When** compiled, **Then** `p.magnitude_sq()` calls the method.
2. **Given** multiple methods in one `impl` block, **When** called, **Then** each method is dispatched correctly.
3. **Given** `impl` on the same struct in different locations, **When** compiled, **Then** all methods are available.

### User Story 2 - Pub Keyword (Priority: P2)

A developer marks items with `pub` to indicate public visibility. Currently parsed but not enforced (enforcement deferred to module system).

**Acceptance Scenarios**:

1. **Given** `pub fn helper() -> i32 { ... }`, **When** compiled, **Then** the function compiles without error.
2. **Given** `pub struct Data { ... }`, **When** compiled, **Then** the struct compiles without error.

## Requirements

- **FR-001**: `impl TypeName { fn method(self: TypeName, ...) -> T { ... } }` defines methods
- **FR-002**: Methods are compiled as `TypeName_method` qualified functions internally
- **FR-003**: Dot syntax `obj.method(args)` dispatches to the qualified function
- **FR-004**: `pub` keyword is recognized on fn, struct, enum, and inside impl blocks
- **FR-005**: `impl Trait for Type { ... }` syntax is parsed (trait conformance deferred)

## Implementation

### AST
- New `Item::ImplBlock { type_name, trait_name: Option<String>, methods: Vec<Item>, span }`

### Compilation
- Pass 0b: Flatten impl methods into qualified names (`TypeName_method`)
- Methods registered in both resolver and typechecker function tables
- Codegen: separate pass for impl method body compilation

### Tests
- `tests/fixtures/valid/impl_block.ny` — Point with `magnitude_sq` and `add` methods (exit code 52)
