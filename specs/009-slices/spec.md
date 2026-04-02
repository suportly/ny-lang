# Feature Specification: Slices (Phase 9)

**Feature Branch**: `004-strings-enums-tuples`
**Created**: 2026-04-01
**Status**: Implemented (type system)

## Summary

Add `[]T` slice type — a borrowed view into contiguous memory represented as `{ptr, len}`. Slices enable functions that operate on arrays of any size without copying.

## User Scenarios & Testing

### User Story 1 - Slice Type Annotation (Priority: P1)

A developer uses `[]T` in function parameters to accept arrays of any size.

**Acceptance Scenarios**:

1. **Given** `fn sum(data: []i32) -> i32`, **When** compiled, **Then** the parameter type is a slice of i32.
2. **Given** `[]T` in a struct field, **When** compiled, **Then** the field is typed as a slice.

## Requirements

- **FR-001**: `[]T` type annotation for slices (no size, unlike `[N]T` arrays)
- **FR-002**: Slice representation is `{ ptr: *T, len: i64 }` (same layout as `str`)
- **FR-003**: Indexing into slices with `slice[i]` returns element type `T`
- **FR-004**: `NyType::Slice(elem)` variant in the type system

## Implementation

### Type System
- New `NyType::Slice(Box<NyType>)` variant
- `is_slice()` method on NyType

### AST
- New `TypeAnnotation::Slice { elem, span }`

### LLVM Mapping
- Slice maps to `{ ptr, i64 }` struct type (identical to str layout)

### Parser
- `[]T` syntax: `[` followed immediately by `]` then element type
