# Feature Specification: Test Framework (Phase 14)

**Feature Branch**: `004-strings-enums-tuples`
**Created**: 2026-04-01
**Status**: Implemented

## Summary

Add a `ny test` CLI subcommand that discovers and runs test functions in a Ny source file. Test functions are identified by the `test_` name prefix and must return `i32` (0 = pass, non-zero = fail).

## User Scenarios & Testing

### User Story 1 - Run Test Functions (Priority: P1)

A developer writes functions prefixed with `test_` and runs them with `ny test file.ny`.

**Acceptance Scenarios**:

1. **Given** a file with `fn test_addition() -> i32 { ... }`, **When** `ny test file.ny` is run, **Then** the function is discovered and executed.
2. **Given** a test that returns 0, **When** run, **Then** it's reported as "ok".
3. **Given** a test that returns non-zero, **When** run, **Then** it's reported as "FAILED".
4. **Given** multiple test functions, **When** run, **Then** all are executed and a summary is printed.
5. **Given** a file with no test functions, **When** run, **Then** "no test functions found" is printed.

## Requirements

- **FR-001**: `ny test file.ny` discovers all functions named `test_*`
- **FR-002**: Each test is compiled as a standalone program with the test function as `main`
- **FR-003**: Test passes if exit code is 0, fails otherwise
- **FR-004**: Output format: `test test_name ... ok` or `test test_name ... FAILED`
- **FR-005**: Summary: `test result: N passed, M failed`
- **FR-006**: Exit code 1 if any test fails, 0 if all pass
- **FR-007**: The existing `fn main()` in the source file is stripped before generating the test wrapper

## Implementation

### CLI
- New `Commands::Test { file: PathBuf }` variant in clap
- Parser discovers `test_*` functions from the AST
- For each test: strip main, append wrapper `fn main() -> i32 { return test_fn(); }`, compile, run
- Cleanup: remove temporary files after each test

### Helper
- `remove_main_function(source) -> String` — text-based removal of existing main function
