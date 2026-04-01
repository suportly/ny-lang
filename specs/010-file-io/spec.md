# Feature Specification: File I/O (Phase 10)

**Feature Branch**: `004-strings-enums-tuples`
**Created**: 2026-04-01
**Status**: Implemented

## Summary

Add file I/O builtins wrapping libc functions: `fopen`, `fclose`, `fwrite_str`, `fread_byte`, and `exit`. These enable Ny programs to interact with the filesystem and control process termination.

## User Scenarios & Testing

### User Story 1 - Write and Read Files (Priority: P1)

A developer opens a file, writes text to it, closes it, then reads it back.

**Acceptance Scenarios**:

1. **Given** `fp := fopen("path\0", "w\0");`, **When** compiled and run, **Then** the file is opened for writing.
2. **Given** `fwrite_str(fp, "hello\0");`, **When** run, **Then** the string is written to the file.
3. **Given** `fread_byte(fp)`, **When** run on an open file, **Then** returns the next byte as i32 (-1 on EOF).
4. **Given** `fclose(fp);`, **When** run, **Then** the file is closed.

### User Story 2 - Process Control (Priority: P1)

A developer uses `exit(code)` to terminate the process immediately.

**Acceptance Scenarios**:

1. **Given** `exit(1);`, **When** run, **Then** the process terminates with exit code 1.

## Requirements

- **FR-001**: `fopen(path: str, mode: str) -> *u8` wraps libc `fopen`
- **FR-002**: `fclose(fp: *u8) -> i32` wraps libc `fclose`
- **FR-003**: `fwrite_str(fp: *u8, data: str) -> i32` wraps libc `fwrite`
- **FR-004**: `fread_byte(fp: *u8) -> i32` wraps libc `fgetc`
- **FR-005**: `exit(code: i32)` wraps libc `exit`
- **FR-006**: String arguments extract the `ptr` from `{ptr, len}` struct for C interop

## Implementation

### Codegen
- Each builtin maps to a libc function declaration (`fopen`, `fclose`, `fwrite`, `fgetc`, `exit`)
- String args: extract field 0 (ptr) from the `{ptr, len}` str struct
- `fwrite_str`: calls `fwrite(ptr, 1, len, fp)`
- `exit`: followed by `unreachable` in LLVM IR

### Tests
- `tests/fixtures/valid/file_io.ny` — write + read + verify first byte (exit code 42)
