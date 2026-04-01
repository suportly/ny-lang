# Feature Specification: Heap Memory & Defer (Phase 5)

**Feature Branch**: `004-strings-enums-tuples`
**Created**: 2026-04-01
**Status**: Implemented

## Summary

Add heap allocation (`alloc`/`free`), compile-time `sizeof`, and `defer` statement for deterministic resource cleanup. This is Ny's first heap-touching feature and the foundation for all dynamic data structures.

## User Scenarios & Testing

### User Story 1 - Allocate and Free Heap Memory (Priority: P1)

A developer allocates a buffer on the heap with `alloc(size)`, uses it, and frees it with `free(ptr)`. The `sizeof` builtin returns the byte size of a type's value.

**Acceptance Scenarios**:

1. **Given** `buf := alloc(64);`, **When** compiled and run, **Then** `buf` is a valid `*u8` pointer to 64 bytes of heap memory.
2. **Given** `free(buf);`, **When** compiled and run, **Then** the memory is released (no crash).
3. **Given** `sizeof(x)` where `x : i32`, **When** evaluated, **Then** the result is `4` (i64).

### User Story 2 - Defer Statement (Priority: P1)

A developer uses `defer` to schedule cleanup code that runs when the enclosing function returns, regardless of which return path is taken. Multiple defers execute in LIFO (last-in, first-out) order.

**Acceptance Scenarios**:

1. **Given** `defer free(buf);`, **When** the function returns, **Then** `free(buf)` is called automatically.
2. **Given** multiple `defer` statements, **When** the function returns, **Then** they execute in reverse declaration order (LIFO).
3. **Given** `defer` before an early `return`, **When** the early return executes, **Then** the deferred expression still runs before the return.

## Requirements

### Functional Requirements

- **FR-001**: `alloc(size: i32) -> *u8` allocates `size` bytes on the heap via `malloc`
- **FR-002**: `free(ptr: *u8)` releases heap memory via `free`
- **FR-003**: `sizeof(expr) -> i64` returns the byte size of the expression's type
- **FR-004**: `defer expr;` schedules `expr` to execute when the current function exits
- **FR-005**: Multiple defers execute in LIFO order
- **FR-006**: Defers execute on all function exit paths (explicit return, implicit end-of-body)

## Implementation

### AST
- New `Stmt::Defer { body: Expr, span: Span }`

### Codegen
- `alloc` → `malloc` FFI call
- `free` → `free` FFI call
- `sizeof` → LLVM `size_of()` on the type
- `defer` → collected in a stack per function; emitted before every `return` and at implicit function end, in reverse order

### Tests
- `tests/fixtures/valid/defer_alloc.ny` — alloc + defer free + return value (exit code 42)
