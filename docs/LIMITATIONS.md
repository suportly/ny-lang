# Ny Lang — Known Limitations

Honest documentation of current constraints. These are design decisions and implementation boundaries, not bugs.

## Concurrency

**Thread pool, not green threads.** `go fn()` dispatches to a fixed-size OS thread pool (`N = CPU cores`). This is efficient for compute-bound workloads with a moderate number of goroutines (tens to low hundreds). It is **not designed** for thousands of lightweight concurrent tasks — for that workload profile, Go's M:N scheduler with green threads is more appropriate.

**Channels are blocking.** `ch.send()` and `ch.recv()` block the OS thread. There is no non-blocking `try_send` in the language-level API (only `try_recv` used internally by `select`).

**`select` uses polling.** The `select` statement polls channels with `try_recv` in a loop with 1ms sleep between rounds, rather than using a true event-driven mechanism. This adds latency for idle waits but works correctly.

## Memory Management

**GC is mark-and-sweep, not generational.** Pauses are proportional to heap size, and `find_object` scans the object list linearly for every marked pointer, so marking is O(n²) in the number of live objects. For latency-sensitive workloads with large heaps (>100MB), consider manual `alloc`/`free` with `defer`.

**The collector is not stop-the-world.** A global mutex keeps the heap structures consistent — concurrent allocations cannot corrupt the object list, and each thread has its own shadow stack, so one thread's roots are never popped by another. But other threads keep running Ny code while a collection marks. If a thread stores a freshly allocated pointer into an already-marked object during the mark, that object can be swept, because nothing re-scans it.

Closing this window requires safepoints: the codegen must emit polls so every thread can be parked at a known point before marking begins. Until that exists, a program that uses `go` together with `new` carries a small risk of a collection freeing a live object. Single-threaded programs are unaffected — the whole collection runs on the only thread there is.

**No escape analysis.** `new Type { ... }` always allocates on the GC heap. The compiler does not promote small, non-escaping allocations to the stack. This means `new` has overhead even for short-lived objects.

**Enum union layout wastes space.** When enum variants have different-sized payloads (e.g., `Ok(i32)` vs `Err(str)`), the struct uses the size of the largest variant for all. An `Ok(i32)` value occupies 16+ bytes instead of 4.

## Type System

**No generic enums with type parameters.** `enum Result<T, E>` with full type parameter support is not implemented. You must define concrete enums: `enum Result { Ok(i32), Err(str) }`.

**`?T` optional types work best with pointers.** For `?*Point`, the optional is represented as a nullable pointer (zero overhead). For value types like `?i32`, the representation is a struct `{bool, i32}` (2x size).

**Monomorphization causes code bloat.** Each generic instantiation (`max<i32>`, `max<f64>`, `max<bool>`) generates separate machine code. For libraries with many generic functions, binary size grows. Use `dyn Trait` for code-size-sensitive paths.

## Error Handling

**Global error table.** `error_new(msg)` stores messages in a static global array (max 1024 entries). Error codes are integers, not typed objects. There is no `Error` trait or error chaining (`caused by`).

**Stack traces only in debug builds.** `error_trace(code)` returns the call stack at error creation, but only when compiled without `-O2` or higher (debug mode). In release builds, trace push/pop is disabled for performance, so `error_trace()` returns an empty string.

## Deprecated Features

**`async`/`await` is deprecated.** The `async fn` syntax exists but dispatches to the same thread pool as `go`. The semantics do not match JavaScript/Rust `async` (no coroutines, no suspension points). Use `go fn() + chan<T>` instead. A compilation warning is emitted when `async`/`await` is used.

## Tooling

**LSP has not been updated for new keywords.** The language server supports diagnostics, hover, goto-definition, completion, and document symbols, but may not fully support `dyn Trait`, `?T`, `chan<T>`, or `select` in all contexts.

**Package manager is basic.** `ny pkg` supports git-based dependencies with SHA pinning, but has no version resolution, no lockfile deduplication, and no registry.
