# Feature Specification: Traits (Phase 8)

**Feature Branch**: `004-strings-enums-tuples`
**Created**: 2026-04-01
**Status**: Implemented (parsing + definition)

## Summary

Add `trait` definitions with method signatures and `impl Trait for Type { ... }` syntax. This phase establishes the trait system's syntax and parsing. Full trait conformance checking and generic trait bounds are deferred to a future phase.

## User Scenarios & Testing

### User Story 1 - Trait Definition (Priority: P1)

A developer defines a trait with method signatures that types can implement.

**Acceptance Scenarios**:

1. **Given** `trait Describable { fn describe(self: i32) -> i32; }`, **When** compiled, **Then** the trait definition is parsed without error.
2. **Given** `impl Describable for Point { ... }`, **When** compiled, **Then** the impl block is parsed and methods are available.

## Requirements

- **FR-001**: `trait Name { fn method(params) -> RetType; ... }` syntax for trait definitions
- **FR-002**: Trait methods are semicolon-terminated signatures (no default bodies in this phase)
- **FR-003**: `impl Trait for Type { fn method(...) { ... } }` syntax for trait implementations
- **FR-004**: Trait definitions are type-checked at the impl site

## Implementation

### AST
- New `Item::TraitDef { name, methods: Vec<TraitMethodSig>, span }`
- New `TraitMethodSig { name, params, return_type, span }`
- `Item::ImplBlock` gains `trait_name: Option<String>` field

### Tests
- `tests/fixtures/valid/traits.ny` — Circle and Square with impl blocks (exit code 52)
