# Tasks: Concurrency Primitives (Phase 15)

**Input**: Design documents from `/specs/015-concurrency-primitives/`
**Prerequisites**: plan.md, spec.md

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (US1=Channels, US2=ThreadPool, US3=ParallelIterators)
- All file paths are relative to repository root

---

## Phase 1: Setup (Builtins Registration)

**Purpose**: Register all new builtin functions in the compiler so they are recognized by resolver, typechecker, and codegen.

- [ ] T001 Add channel/pool/par builtin names to BUILTIN_NAMES in src/codegen/builtins.rs
- [ ] T002 Add channel/pool/par return types to builtin_return_type() in src/codegen/builtins.rs
- [ ] T003 [P] Add channel/pool/par type checking in src/semantic/typechecker.rs (channel_new -> *u8, channel_recv -> i32, etc.)

---

## Phase 2: Foundational (C Runtime Stubs)

**Purpose**: Create the C runtime files with correct signatures so linking works. Actual implementation follows in user story phases.

**CRITICAL**: These files must exist before any user story codegen can link.

- [ ] T004 Create runtime/channel.c with struct definitions and function stubs (ny_channel_new, ny_channel_send, ny_channel_recv, ny_channel_close)
- [ ] T005 [P] Create runtime/threadpool.c with struct definitions and function stubs (ny_pool_new, ny_pool_submit, ny_pool_wait, ny_pool_free, ny_par_map, ny_par_reduce)
- [ ] T006 Verify linker includes runtime/channel.c and runtime/threadpool.c — update link list in src/codegen/mod.rs link_executable()

**Checkpoint**: Compiler compiles .ny files that call channel/pool builtins without link errors (stubs return 0/NULL).

---

## Phase 3: User Story 1 — Channels (Priority: P1) MVP

**Goal**: Buffered channels for thread-safe message passing between threads.

**Independent Test**: Spawn a thread that sends 5 values to a channel, receive all 5 in main thread, verify sum.

### Implementation for User Story 1

- [ ] T007 [US1] Implement NyChannel struct in runtime/channel.c: ring buffer (int32_t*), head, tail, count, capacity, closed flag, pthread_mutex_t, two pthread_cond_t (not_empty, not_full)
- [ ] T008 [US1] Implement ny_channel_new(capacity) in runtime/channel.c: malloc struct + buffer, init mutex + condvars, return pointer
- [ ] T009 [US1] Implement ny_channel_send(ch, value) in runtime/channel.c: lock, while(count==capacity && !closed) wait(not_full), enqueue at tail, signal(not_empty), unlock
- [ ] T010 [US1] Implement ny_channel_recv(ch) in runtime/channel.c: lock, while(count==0 && !closed) wait(not_empty), if(closed && count==0) return 0, dequeue at head, signal(not_full), unlock
- [ ] T011 [US1] Implement ny_channel_close(ch) in runtime/channel.c: lock, set closed=1, broadcast both condvars, unlock
- [ ] T012 [US1] Add codegen for channel_new/send/recv/close in src/codegen/mod.rs: declare C functions, compile builtin calls to LLVM call instructions
- [ ] T013 [US1] Create test fixture tests/fixtures/valid/channel.ny: spawn producer thread sending values 1-5 to channel, main thread receives and sums, verify sum=15 (exit code 15+27=42)
- [ ] T014 [US1] Add test_channel to tests/compile_run.rs and verify it passes

**Checkpoint**: Channels work end-to-end — producer/consumer across threads with correct synchronization.

---

## Phase 4: User Story 2 — Thread Pool (Priority: P2)

**Goal**: Fixed-size thread pool for efficient parallel work execution.

**Independent Test**: Create pool of 4, submit 8 work items that each send to a channel, verify all 8 received.

### Implementation for User Story 2

- [ ] T015 [US2] Implement WorkItem struct and work queue in runtime/threadpool.c: function pointer, queue array, head/tail/count/capacity
- [ ] T016 [US2] Implement worker_thread() loop in runtime/threadpool.c: lock, while(!shutdown && queue_empty) wait, dequeue, unlock, execute, decrement active_count, signal all_done
- [ ] T017 [US2] Implement ny_pool_new(n) in runtime/threadpool.c: malloc struct, init queue, create n pthreads running worker_thread
- [ ] T018 [US2] Implement ny_pool_submit(pool, fn) in runtime/threadpool.c: lock, enqueue work item, signal work_available, unlock
- [ ] T019 [US2] Implement ny_pool_wait(pool) in runtime/threadpool.c: lock, while(queue_count > 0 || active_count > 0) wait(all_done), unlock
- [ ] T020 [US2] Implement ny_pool_free(pool) in runtime/threadpool.c: set shutdown=1, broadcast, join all threads, free resources
- [ ] T021 [US2] Add codegen for pool_new/submit/wait/free in src/codegen/mod.rs
- [ ] T022 [US2] Create test fixture tests/fixtures/valid/threadpool.ny: create pool(4), submit 8 tasks via channel to verify completion, exit code 42
- [ ] T023 [US2] Add test_threadpool to tests/compile_run.rs and verify it passes

**Checkpoint**: Thread pool executes submitted work items across multiple threads correctly.

---

## Phase 5: User Story 3 — Parallel Iterators (Priority: P3)

**Goal**: High-level par_map and par_reduce that distribute array processing across a thread pool.

**Independent Test**: par_map squaring 100 elements produces same result as sequential map.

### Implementation for User Story 3

- [ ] T024 [US3] Implement ny_par_map(data, n, result, fn, pool) in runtime/threadpool.c: divide n into chunks (n/num_threads), submit chunk processing as work items, wait for completion
- [ ] T025 [US3] Implement ny_par_reduce(data, n, init, fn, pool) in runtime/threadpool.c: divide into chunks, reduce each chunk, then reduce partial results sequentially
- [ ] T026 [US3] Add codegen for par_map/par_reduce in src/codegen/mod.rs
- [ ] T027 [US3] Create test fixture tests/fixtures/valid/par_map.ny: allocate array of 100 i32, fill with 1..100, par_map with squaring fn, verify sum matches sequential
- [ ] T028 [US3] Add test_par_map to tests/compile_run.rs and verify it passes

**Checkpoint**: Parallel iterators produce correct results and distribute work across threads.

---

## Phase 6: Polish & Cross-Cutting Concerns

**Purpose**: Documentation, cleanup, regression testing.

- [ ] T029 [P] Run full test suite (cargo test) — verify all 66+ existing tests still pass
- [ ] T030 [P] Run cargo clippy — fix any new warnings
- [ ] T031 Update CLAUDE.md with new builtins (channel_*, pool_*, par_*)
- [ ] T032 Update specs/ROADMAP.md to mark concurrency phase as implemented
- [ ] T033 Update examples/benchmark/main.ny to include a channel + pool demonstration

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies — start immediately
- **Foundational (Phase 2)**: Depends on Phase 1 — creates C stubs for linking
- **Channels (Phase 3)**: Depends on Phase 2 — first independently testable feature
- **Thread Pool (Phase 4)**: Depends on Phase 2 — can use channels for testing
- **Parallel Iterators (Phase 5)**: Depends on Phase 4 — requires working thread pool
- **Polish (Phase 6)**: Depends on all user stories being complete

### User Story Dependencies

- **US1 (Channels)**: Independent — can start after Phase 2
- **US2 (Thread Pool)**: Independent of US1 for implementation, but uses channels in tests
- **US3 (Parallel Iterators)**: Depends on US2 (uses thread pool internally)

### Within Each User Story

- C runtime implementation → compiler codegen → test fixture → integration test
- Each task builds on the previous within its story

### Parallel Opportunities

- T001 and T003 can run in parallel (different files)
- T004 and T005 can run in parallel (different runtime files)
- T029 and T030 can run in parallel (independent checks)
- US1 and US2 implementation can overlap (different C files)

---

## Parallel Example: Phase 1

```bash
# These can run simultaneously:
Task T001: "Add builtin names to BUILTIN_NAMES in src/codegen/builtins.rs"
Task T003: "Add type checking in src/semantic/typechecker.rs"
```

## Parallel Example: Phase 2

```bash
# These can run simultaneously:
Task T004: "Create runtime/channel.c with stubs"
Task T005: "Create runtime/threadpool.c with stubs"
```

---

## Implementation Strategy

### MVP First (Channels Only)

1. Complete Phase 1: Setup (T001-T003)
2. Complete Phase 2: Foundational (T004-T006)
3. Complete Phase 3: Channels (T007-T014)
4. **STOP and VALIDATE**: Test channels independently
5. Channels alone deliver value — safe thread communication

### Incremental Delivery

1. Setup + Foundational → Compiler recognizes all builtins
2. Add Channels → Test independently → First concurrent Ny program!
3. Add Thread Pool → Test independently → Efficient parallel execution
4. Add Parallel Iterators → Test independently → High-level parallelism API
5. Each phase adds concurrency capability without breaking previous features

---

## Notes

- All C runtime files use pthreads (linked via -lpthread, already in linker flags)
- Channel only supports i32 values (matching Vec pattern)
- Thread pool work items use `void* (*fn)(void*)` signature (pthread compatible)
- par_map/par_reduce require caller-allocated buffers
- Commit after each task or logical group
- Run `cargo test` after each phase checkpoint
