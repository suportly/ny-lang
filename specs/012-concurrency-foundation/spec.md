# Feature Specification: Concurrency Foundation (Phase 12)

**Feature Branch**: `004-strings-enums-tuples`
**Created**: 2026-04-01
**Status**: Implemented (foundation)

## Summary

Add foundational concurrency primitives: `sleep_ms` for timing control. The `loop` keyword (Phase 6) and `unsafe` keyword are reserved for future thread-safety features. Full threading (pthread_create) and channels are deferred to a future phase.

## User Scenarios & Testing

### User Story 1 - Timing Control (Priority: P2)

A developer uses `sleep_ms(n)` to pause execution for `n` milliseconds.

**Acceptance Scenarios**:

1. **Given** `sleep_ms(100);`, **When** run, **Then** execution pauses for approximately 100ms.

## Requirements

- **FR-001**: `sleep_ms(ms: i32)` pauses for the given number of milliseconds (wraps `usleep(ms * 1000)`)
- **FR-002**: Keywords `unsafe`, `loop` are reserved for future concurrency features

## Implementation

- `sleep_ms` → `usleep` libc FFI call with millisecond-to-microsecond conversion
