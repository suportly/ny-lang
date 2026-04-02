# Feature Specification: Concurrency Primitives (Phase 15)

**Feature Branch**: `015-concurrency-primitives`
**Created**: 2026-04-02
**Status**: Draft
**Input**: User description: "Ny Lang Concurrency Roadmap — structured concurrency primitives inspired by Go channels and Java Loom. Channels, thread pool, parallel iterators."

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Send and Receive Messages Between Threads (Priority: P1)

A developer creates a channel, spawns a producer thread that sends values, and receives them in the main thread. This is the fundamental concurrency building block — without it, threads can only communicate via shared memory (unsafe).

**Why this priority**: Channels are the foundation for all safe concurrent communication. Every other concurrency feature (pool, parallel iterators) builds on top of message passing.

**Independent Test**: Create a channel, spawn a thread that sends 5 integers, receive all 5 in main thread, verify sum is correct.

**Acceptance Scenarios**:

1. **Given** `ch := channel_new(16);`, **When** compiled and run, **Then** a buffered channel with capacity 16 is created.
2. **Given** a producer thread calling `channel_send(ch, 42);`, **When** the main thread calls `val := channel_recv(ch);`, **Then** `val` is 42.
3. **Given** a channel with capacity 1 that is full, **When** a sender calls `channel_send`, **Then** the sender blocks until the receiver consumes the value.
4. **Given** a channel that is empty, **When** a receiver calls `channel_recv`, **Then** the receiver blocks until a sender provides a value.
5. **Given** `channel_close(ch);`, **When** a receiver calls `channel_recv` on a closed empty channel, **Then** it returns 0 (sentinel value for closed channel).

---

### User Story 2 - Submit Work to a Thread Pool (Priority: P2)

A developer creates a fixed-size thread pool, submits multiple work items, and waits for all to complete. This avoids the overhead of creating and destroying OS threads for each task.

**Why this priority**: Thread pools enable efficient parallelism without the cost of thread creation per task. Critical for ML workloads that process batches of data.

**Independent Test**: Create a pool of 4 threads, submit 8 work items that each send a result to a channel, verify all 8 completed.

**Acceptance Scenarios**:

1. **Given** `pool := pool_new(4);`, **When** compiled, **Then** a pool with 4 worker threads is created.
2. **Given** `pool_submit(pool, work_fn);` called 8 times, **When** `pool_wait(pool);` returns, **Then** all 8 functions have executed to completion.
3. **Given** a pool with work in progress, **When** `pool_wait(pool);` is called, **Then** it blocks until all submitted work is done.
4. **Given** a pool, **When** `pool_free(pool);` is called, **Then** all worker threads are shut down and resources freed.

---

### User Story 3 - Parallel Map Over Data (Priority: P3)

A developer applies a transformation function to each element of an array in parallel using a thread pool, collecting results. This is the highest-level parallelism API — write `par_map` instead of manually managing threads.

**Why this priority**: Parallel iterators are the user-facing API that makes concurrency accessible. They build on channels and thread pool, so they come last.

**Independent Test**: Create an array of 1000 integers, apply `par_map` with a squaring function, verify all results are correct.

**Acceptance Scenarios**:

1. **Given** an array of N i32 values and a function `fn(i32) -> i32`, **When** `par_map(data_ptr, n, result_ptr, square_fn, pool);` is called, **Then** each element is transformed in parallel and stored in result array.
2. **Given** `par_reduce(data_ptr, n, 0, add_fn, pool);`, **When** called on an array, **Then** all elements are reduced to a single value using the add function.
3. **Given** a pool with 4 threads and 1000 elements, **When** par_map is called, **Then** work is distributed across all 4 threads approximately evenly.

---

### Edge Cases

- What happens when sending to a closed channel? The send is a no-op (value is dropped).
- What happens when receiving from a channel that was never sent to? The receiver blocks indefinitely (deadlock — caller's responsibility to avoid).
- What happens when pool_submit is called after pool_free? Undefined behavior (caller's responsibility).
- What happens when par_map is called with n=0? Returns immediately with no work done.
- What happens when the pool size is 0? Created with 1 thread minimum.
- What happens when multiple threads send to the same channel simultaneously? The ring buffer and mutex ensure thread-safe FIFO ordering.

## Requirements *(mandatory)*

### Functional Requirements

**Channels**
- **FR-001**: System MUST support `channel_new(capacity)` creating a buffered channel backed by a ring buffer
- **FR-002**: System MUST support `channel_send(ch, value)` that blocks when the buffer is full
- **FR-003**: System MUST support `channel_recv(ch)` that blocks when the buffer is empty and returns the next value
- **FR-004**: System MUST support `channel_close(ch)` that signals no more values will be sent
- **FR-005**: `channel_recv` on a closed, empty channel MUST return 0 (sentinel)
- **FR-006**: Channel operations MUST be thread-safe using synchronization primitives

**Thread Pool**
- **FR-007**: System MUST support `pool_new(num_threads)` creating a fixed-size thread pool
- **FR-008**: System MUST support `pool_submit(pool, fn)` adding work to the pool's queue
- **FR-009**: System MUST support `pool_wait(pool)` blocking until all submitted work completes
- **FR-010**: System MUST support `pool_free(pool)` shutting down all threads and freeing resources
- **FR-011**: Work items MUST be executed in FIFO order within each thread

**Parallel Iterators**
- **FR-012**: System MUST support `par_map(data, n, result, fn, pool)` applying a function to each element in parallel
- **FR-013**: System MUST support `par_reduce(data, n, init, fn, pool)` reducing an array in parallel
- **FR-014**: Parallel operations MUST distribute work approximately evenly across pool threads

### Key Entities

- **Channel**: A bounded FIFO queue with blocking send/recv. Holds i32 values. Created with a fixed capacity.
- **Thread Pool**: A fixed set of worker threads that pull work from a shared queue. Workers sleep when idle and wake on new work.
- **Work Item**: A function pointer submitted to the pool for execution. Executed exactly once by one worker thread.
- **Parallel Iterator**: A high-level operation that divides an array into chunks, submits each chunk as work to the pool, and collects results.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: A producer-consumer program using channels correctly transfers 1000 values between threads without data loss or corruption.
- **SC-002**: A thread pool with 4 workers completes 100 submitted tasks, with each task executed exactly once.
- **SC-003**: `par_map` on a 10000-element array produces identical results to sequential map.
- **SC-004**: `par_map` with 4 threads achieves measurable speedup on compute-heavy workloads compared to sequential execution.
- **SC-005**: All existing tests (66) continue to pass without regression.
- **SC-006**: At least 4 new integration tests cover channels, pool, and parallel operations.
- **SC-007**: No memory leaks when channels and pools are properly freed.

## Assumptions

- Thread-safety is the caller's responsibility for shared mutable state outside channels. Channels are the safe communication mechanism.
- Channels only support `i32` values in this phase. Generic channels require generic types not yet available.
- Thread pool uses OS threads (pthreads), not green threads. One OS thread per pool worker.
- `par_map` and `par_reduce` require caller-allocated result buffers. They do not allocate memory internally.
- Platform is x86-64 Linux with pthreads available.
- `pool_submit` accepts function pointers with pthread-compatible signature. The caller wraps closures or parameterized work into this signature.
- Channel implementation uses a fixed-size ring buffer. If capacity is exceeded, sender blocks. There is no non-blocking variant in this phase.
- Phase 1 (thread_spawn/thread_join) is already implemented and functional.
