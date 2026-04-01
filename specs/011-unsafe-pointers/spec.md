# Feature Specification: Unsafe Pointer Operations (Phase 11)

**Feature Branch**: `004-strings-enums-tuples`
**Created**: 2026-04-01
**Status**: Implemented

## Summary

Enable raw pointer manipulation: allocate heap memory, write through pointers, and cast between types. Combined with Phase 5's `alloc`/`free`, this enables manual memory management patterns.

## User Scenarios & Testing

### User Story 1 - Raw Pointer Write (Priority: P1)

A developer allocates memory, writes a value through a dereferenced pointer, and reads it back.

**Acceptance Scenarios**:

1. **Given** `buf := alloc(32); *buf = 42 as u8;`, **When** run, **Then** the byte 42 is stored at the buffer address.
2. **Given** `val := *buf;`, **When** run after writing, **Then** `val` equals the stored byte.
3. **Given** `val as i32`, **When** evaluated, **Then** the byte is widened to an i32.

## Requirements

- **FR-001**: `*ptr = value` writes through a raw pointer
- **FR-002**: `*ptr` reads through a raw pointer
- **FR-003**: `expr as Type` casts between pointer-compatible types
- **FR-004**: `unsafe` keyword is reserved (parsed, not yet enforced)

## Tests
- `tests/fixtures/valid/unsafe_ptr.ny` — alloc + deref write + cast + defer free (exit code 42)
