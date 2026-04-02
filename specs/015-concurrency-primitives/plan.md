# Implementation Plan: Concurrency Primitives (Phase 15)

**Branch**: `015-concurrency-primitives` | **Date**: 2026-04-02 | **Spec**: [spec.md](spec.md)

## Summary

Add concurrency primitives to Ny Lang: buffered channels for thread-safe message passing, a fixed-size thread pool for reusable parallel execution, and parallel iterators (par_map/par_reduce) for high-level data parallelism. All backed by C runtime libraries using pthreads.

## Technical Context

**Language/Version**: Rust 1.75+ (2021 edition)
**Primary Dependencies**: inkwell (LLVM 18), pthreads (libc)
**Runtime**: C files in runtime/ auto-linked by the compiler
**Testing**: cargo test (integration tests compile .ny files and verify exit codes/stdout)
**Target Platform**: x86-64 Linux
**Project Type**: Compiler + C runtime
**Constraints**: No GC, manual memory + arena allocators, sync threads only (no async)

## Project Structure

### New Files

```text
runtime/
├── channel.c          # Bounded channel: ring buffer + mutex + condvar
└── threadpool.c       # Thread pool: work queue + worker threads

src/
├── codegen/
│   ├── builtins.rs    # EXTEND: add channel/pool/par builtins
│   └── mod.rs         # EXTEND: codegen for new builtins
└── semantic/
    └── typechecker.rs # EXTEND: type-check new builtins

tests/fixtures/valid/
├── channel.ny         # Channel send/recv test
├── threadpool.ny      # Pool submit/wait test
└── par_map.ny         # Parallel map test
```

### Existing Files (Modified)

- `src/codegen/builtins.rs` — add 9 new builtins to registry
- `src/codegen/mod.rs` — add codegen for channel/pool/par builtins, add libc declarations
- `src/semantic/typechecker.rs` — add type checking for new builtins

## Implementation Strategy

### Phase 2: Channels (P1 — independently testable)

**C Runtime** (`runtime/channel.c`):
- `NyChannel` struct: `{ int32_t *buffer, int head, int tail, int count, int capacity, int closed, pthread_mutex_t mutex, pthread_cond_t not_empty, pthread_cond_t not_full }`
- `ny_channel_new(capacity)` → malloc + init mutex/condvar
- `ny_channel_send(ch, value)` → lock, wait if full, enqueue, signal not_empty, unlock
- `ny_channel_recv(ch)` → lock, wait if empty (return 0 if closed), dequeue, signal not_full, unlock
- `ny_channel_close(ch)` → set closed flag, broadcast both condvars

**Compiler**:
- Add `channel_new`, `channel_send`, `channel_recv`, `channel_close` to builtins
- All return opaque pointer (*u8) except recv (i32) and send/close (void)

### Phase 3: Thread Pool (P2 — depends on Phase 2 for testing)

**C Runtime** (`runtime/threadpool.c`):
- `NyPool` struct: `{ pthread_t *threads, WorkItem *queue, int queue_head, int queue_tail, int queue_count, int queue_cap, int active_count, int shutdown, pthread_mutex_t mutex, pthread_cond_t work_available, pthread_cond_t all_done }`
- `WorkItem`: `{ void* (*fn)(void*), void *arg }`
- Worker loop: lock → wait for work → dequeue → unlock → execute → decrement active_count
- `ny_pool_new(n)` → create n pthreads running worker loop
- `ny_pool_submit(pool, fn)` → enqueue work item, signal work_available
- `ny_pool_wait(pool)` → wait until queue empty AND active_count == 0
- `ny_pool_free(pool)` → set shutdown, broadcast, join all threads, free

### Phase 4: Parallel Iterators (P3 — depends on Phase 3)

**C Runtime** (in `runtime/threadpool.c`):
- `ny_par_map(data, n, result, fn, pool)` → divide n elements into chunks, submit each chunk as work item, wait
- `ny_par_reduce(data, n, init, fn, pool)` → divide into chunks, reduce each chunk, then reduce partial results

## Dependency Graph

```
Phase 1: Setup (builtins registration)
    ↓
Phase 2: Channels (runtime/channel.c + codegen)  ← independently testable
    ↓
Phase 3: Thread Pool (runtime/threadpool.c + codegen)  ← testable with channels
    ↓
Phase 4: Parallel Iterators (extends threadpool.c + codegen)  ← testable with pool
    ↓
Phase 5: Polish (tests, docs)
```

## Complexity Tracking

No major risks. All concurrency primitives are well-understood patterns (bounded buffer, thread pool, fork-join). C runtime approach avoids LLVM IR complexity.
