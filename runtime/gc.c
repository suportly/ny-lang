// Ny Lang runtime: tracing mark-and-sweep garbage collector
//
// A stop-the-world mark-and-sweep collector with per-thread shadow stacks for
// precise root enumeration. Mutator threads poll ny_gc_stw_requested on loop
// back-edges and park at ny_gc_safepoint; operations that block without
// reaching a poll bracket themselves with ny_gc_park/ny_gc_unpark. Marking
// only begins once every participating thread is parked.
//
// Memory layout of a GC-managed object:
//   [ NyGcObject header | payload (user data) ]
//   ^                    ^
//   |                    returned to caller
//   managed internally
//
// The shadow stack tracks pointers to stack slots that hold GC pointers.
// Generated code pushes/pops entries as functions are entered/exited.

#include "gc.h"
#include <stdlib.h>
#include <string.h>
#include <stdio.h>
#include <pthread.h>

// ---------------------------------------------------------------------------
// Global heap
// ---------------------------------------------------------------------------
//
// g_heap and everything reachable from it — the object list, the counters and
// the list of per-thread shadow stacks — are guarded by g_heap_mutex.
// Allocation and collection both take it, so a collection never runs while
// another thread is splicing an object into the list.
//
// Root push/pop stay off the lock: they only touch the calling thread's own
// shadow stack. The one exception is the first push on a thread, which links
// that stack into the global list and does take the lock.

static NyGcHeap g_heap;
static int g_initialized = 0;
static pthread_mutex_t g_heap_mutex = PTHREAD_MUTEX_INITIALIZER;

// ---------------------------------------------------------------------------
// Stop-the-world handshake
// ---------------------------------------------------------------------------
//
// g_stw_mutex guards the participation bookkeeping below. It is deliberately
// separate from g_heap_mutex: a collector holding the heap lock has to wait
// for other threads to park, and those threads must be able to park without
// contending for that same lock. Where both are needed the order is always
// g_stw_mutex then g_heap_mutex, never the reverse.

volatile int ny_gc_stw_requested = 0;

static pthread_mutex_t g_stw_mutex = PTHREAD_MUTEX_INITIALIZER;
static pthread_cond_t g_all_parked = PTHREAD_COND_INITIALIZER;
static pthread_cond_t g_resume = PTHREAD_COND_INITIALIZER;

// Threads that participate in the handshake, and how many of those are
// currently running Ny code rather than parked.
static int g_participants = 0;
static int g_running = 0;

// Every participating thread is in exactly one of two states, and g_running
// counts those in NY_RUNNING. Keeping it to a single variable — rather than
// several flags that have to agree — is what makes the transitions below
// checkable by inspection.
#define NY_OUTSIDE 0   // not participating
#define NY_RUNNING 1   // running Ny code, counted in g_running
#define NY_PARKED  2   // at a safepoint or blocked, not counted

static __thread int t_state = NY_OUTSIDE;

// Nesting depth of ny_gc_park, so nested blocking sites stay balanced.
static __thread int t_park_depth = 0;

// Debug-only invariant check: g_running must equal the number of threads in
// NY_RUNNING. Build the runtime with -DNY_GC_CHECK_INVARIANT to have every
// transition verified, aborting at the point the invariant breaks rather than
// minutes later at a deadlock.
#ifdef NY_GC_CHECK_INVARIANT
static int g_dbg_running_states[256];
static int g_dbg_next_slot = 0;
static __thread int t_dbg_slot = -1;

static void dbg_set_state(int st) {
    if (t_dbg_slot < 0) {
        t_dbg_slot = __sync_fetch_and_add(&g_dbg_next_slot, 1);
    }
    if (t_dbg_slot < 256) g_dbg_running_states[t_dbg_slot] = st;
}

// Caller holds g_stw_mutex.
static void dbg_check(const char *where) {
    int counted = 0;
    for (int i = 0; i < g_dbg_next_slot && i < 256; i++) {
        if (g_dbg_running_states[i] == NY_RUNNING) counted++;
    }
    if (counted != g_running) {
        fprintf(stderr,
                "ny: GC invariant violated at %s: g_running=%d but %d threads "
                "are RUNNING\n", where, g_running, counted);
        abort();
    }
    if (g_running < 0) {
        fprintf(stderr, "ny: GC g_running went negative at %s\n", where);
        abort();
    }
}
#else
#define dbg_set_state(st) ((void)0)
#define dbg_check(where)  ((void)0)
#endif

// Per-thread shadow stack. Zero-initialised, so a thread that never roots
// anything costs nothing and is never registered.
static __thread NyGcShadowStack t_roots;

// Key whose only job is to run a destructor when a thread exits, so its
// shadow stack is unlinked before the thread's storage goes away.
static pthread_key_t g_thread_exit_key;
static pthread_once_t g_thread_exit_once = PTHREAD_ONCE_INIT;

static void unregister_thread_roots(void *unused);
static void join_stw(void);
static void leave_stw(void);
static void heap_lock(void);
static void heap_unlock(void);

static void make_thread_exit_key(void) {
    pthread_key_create(&g_thread_exit_key, unregister_thread_roots);
}

// Default threshold: trigger collection after 1MB of allocations
#define NY_GC_DEFAULT_THRESHOLD (1024 * 1024)

// Growth factor: after collection, set threshold to 2x live data
#define NY_GC_GROWTH_FACTOR 2

// ---------------------------------------------------------------------------
// Init / Shutdown
// ---------------------------------------------------------------------------

// Caller must hold g_heap_mutex.
static void gc_init_locked(void) {
    if (g_initialized) return;
    g_heap.objects = NULL;
    g_heap.bytes_allocated = 0;
    g_heap.threshold = NY_GC_DEFAULT_THRESHOLD;
    g_heap.collections = 0;
    g_heap.total_freed = 0;
    g_heap.thread_roots = NULL;

    g_initialized = 1;
}

void ny_gc_init(void) {
    pthread_mutex_lock(&g_heap_mutex);
    gc_init_locked();
    pthread_mutex_unlock(&g_heap_mutex);
}

void ny_gc_shutdown(void) {
    pthread_mutex_lock(&g_heap_mutex);
    if (!g_initialized) {
        pthread_mutex_unlock(&g_heap_mutex);
        return;
    }

    // Free all remaining objects
    NyGcObject *obj = g_heap.objects;
    while (obj) {
        NyGcObject *next = obj->next;
        free(obj);
        obj = next;
    }

    // Thread shadow stacks are owned by their threads; just drop the list.
    // Each entries array is released by the thread's exit destructor.
    g_heap.objects = NULL;
    g_heap.bytes_allocated = 0;
    g_heap.collections = 0;
    g_heap.total_freed = 0;
    g_heap.thread_roots = NULL;
    g_initialized = 0;
    pthread_mutex_unlock(&g_heap_mutex);
}

// ---------------------------------------------------------------------------
// Shadow stack operations
// ---------------------------------------------------------------------------

// Unlink a dead thread's shadow stack and release it. Runs as the thread's
// pthread key destructor, so the collector never walks freed storage.
static void unregister_thread_roots(void *unused) {
    (void)unused;
    if (!t_roots.registered) return;

    // Stop counting towards the handshake before the stack goes away, so a
    // collector is not left waiting on a thread that has exited.
    leave_stw();

    pthread_mutex_lock(&g_heap_mutex);
    NyGcShadowStack **link = &g_heap.thread_roots;
    while (*link) {
        if (*link == &t_roots) {
            *link = t_roots.next;
            break;
        }
        link = &(*link)->next;
    }
    pthread_mutex_unlock(&g_heap_mutex);

    free(t_roots.entries);
    t_roots.entries = NULL;
    t_roots.count = 0;
    t_roots.capacity = 0;
    t_roots.next = NULL;
    t_roots.registered = 0;
}

// Link this thread's shadow stack into the global list on first use.
static void register_thread_roots(void) {
    t_roots.capacity = NY_GC_SHADOW_STACK_MAX;
    t_roots.entries = (void **)calloc(NY_GC_SHADOW_STACK_MAX, sizeof(void *));
    if (!t_roots.entries) {
        fprintf(stderr, "ny: out of memory allocating GC shadow stack\n");
        abort();
    }
    t_roots.count = 0;

    // Arrange for the destructor to run when this thread exits. The value is
    // only a non-NULL marker; the data itself lives in the __thread struct.
    pthread_once(&g_thread_exit_once, make_thread_exit_key);
    pthread_setspecific(g_thread_exit_key, (void *)&t_roots);

    heap_lock();
    gc_init_locked();
    t_roots.next = g_heap.thread_roots;
    g_heap.thread_roots = &t_roots;
    t_roots.registered = 1;
    heap_unlock();

    // From here on this thread has roots the collector must see, so it takes
    // part in the stop-the-world handshake.
    join_stw();
}

void ny_gc_root_push(void **slot) {
    if (!t_roots.registered) {
        register_thread_roots();
    }

    if (t_roots.count >= t_roots.capacity) {
        // Grow this thread's shadow stack. The collector reads `entries`
        // under the heap lock, so swap it there rather than in place.
        int64_t new_cap = t_roots.capacity * 2;
        void **new_entries = (void **)malloc(new_cap * sizeof(void *));
        if (!new_entries) {
            fprintf(stderr, "ny: GC shadow stack overflow (%lld roots)\n",
                    (long long)t_roots.count);
            abort();
        }
        memcpy(new_entries, t_roots.entries, t_roots.count * sizeof(void *));

        heap_lock();
        void **old_entries = t_roots.entries;
        t_roots.entries = new_entries;
        t_roots.capacity = new_cap;
        heap_unlock();

        free(old_entries);
    }

    t_roots.entries[t_roots.count++] = slot;
}

void ny_gc_root_pop(int64_t n) {
    t_roots.count -= n;
    if (t_roots.count < 0) t_roots.count = 0;
}

// ---------------------------------------------------------------------------
// Safepoints
// ---------------------------------------------------------------------------
//
// Transitions all happen under g_stw_mutex and preserve one invariant:
// g_running is the number of participating threads in NY_RUNNING.

// Caller holds g_stw_mutex.
static void to_parked(void) {
    if (t_state != NY_RUNNING) return;
    t_state = NY_PARKED;
    dbg_set_state(NY_PARKED);
    g_running--;
    if (g_running == 0) {
        pthread_cond_broadcast(&g_all_parked);
    }
    dbg_check("to_parked");
}

// Caller holds g_stw_mutex. Waits out any collection before running again.
static void to_running(void) {
    if (t_state != NY_PARKED) return;
    while (ny_gc_stw_requested) {
        pthread_cond_wait(&g_resume, &g_stw_mutex);
    }
    t_state = NY_RUNNING;
    dbg_set_state(NY_RUNNING);
    g_running++;
    dbg_check("to_running");
}

// Join the handshake. Called once per thread, from register_thread_roots.
static void join_stw(void) {
    pthread_mutex_lock(&g_stw_mutex);
    g_participants++;
    // Enter parked and let to_running wait out any collection in progress,
    // rather than joining as running and making a collector wait for a
    // safepoint this thread has no reason to reach.
    t_state = NY_PARKED;
    to_running();
    pthread_mutex_unlock(&g_stw_mutex);
}

static void leave_stw(void) {
    if (t_state == NY_OUTSIDE) return;
    pthread_mutex_lock(&g_stw_mutex);
    to_parked();
    g_participants--;
    t_state = NY_OUTSIDE;
    dbg_set_state(NY_OUTSIDE);
    if (g_running == 0) {
        pthread_cond_broadcast(&g_all_parked);
    }
    dbg_check("leave_stw");
    pthread_mutex_unlock(&g_stw_mutex);
}

void ny_gc_safepoint(void) {
    if (t_state != NY_RUNNING) return;

    pthread_mutex_lock(&g_stw_mutex);
    if (ny_gc_stw_requested) {
        to_parked();
        to_running();
    }
    pthread_mutex_unlock(&g_stw_mutex);
}

// The depth is counted unconditionally, never gated on the thread's state.
// A thread can join the handshake between a park and its unpark — a pool
// worker takes its first root inside the goroutine it is already running — and
// gating on state would let an unpark run to_running() with no matching park,
// permanently inflating g_running and stalling every later collection.
//
// to_parked/to_running are no-ops unless the thread is in the matching state,
// so an unbalanced state costs nothing here.
void ny_gc_park(void) {
    pthread_mutex_lock(&g_stw_mutex);
    if (t_park_depth++ == 0) {
        to_parked();
    }
    pthread_mutex_unlock(&g_stw_mutex);
}

void ny_gc_unpark(void) {
    pthread_mutex_lock(&g_stw_mutex);
    if (t_park_depth > 0 && --t_park_depth == 0) {
        to_running();
    }
    pthread_mutex_unlock(&g_stw_mutex);
}

// Acquire the heap lock from a thread that might be counted as running.
//
// Blocking on a mutex is not a safepoint: the collector waits for every
// running thread to park, and a thread stuck acquiring a lock never reaches a
// poll. Park across the wait so the handshake can always complete, and only
// rejoin the running set once the lock is held.
//
// The unpark inside cannot deadlock on the collection: ny_gc_collect never
// holds this lock (the world is stopped, so it does not need to), so the
// thread that owns it will release it independently of any collection.
// Take the heap lock without ever blocking on it while a collection wants to
// run.
//
// Parking across the wait is not an option: a parked thread is one the
// collector may mark and sweep alongside, but a thread holding this lock is
// mutating the object list. So instead of parking, reach a safepoint first —
// which yields to any pending collection — and only then contend for the lock.
// The window between the safepoint and acquiring the lock is bounded: the
// collector holds this same lock, so a collection that starts in that window
// simply makes this thread wait until it finishes.
static void heap_lock(void) {
    ny_gc_safepoint();
    pthread_mutex_lock(&g_heap_mutex);
}

static void heap_unlock(void) {
    pthread_mutex_unlock(&g_heap_mutex);
}


// Stop every other participating thread. The collector stays parked for the
// whole call — including while waiting its turn behind another collection —
// so two threads that both want to collect can never wait on each other.
static void stw_begin(void) {
    pthread_mutex_lock(&g_stw_mutex);

    to_parked();
    while (ny_gc_stw_requested) {
        pthread_cond_wait(&g_resume, &g_stw_mutex);
    }
    ny_gc_stw_requested = 1;

    while (g_running > 0) {
        pthread_cond_wait(&g_all_parked, &g_stw_mutex);
    }
    dbg_check("stw_begin");
    pthread_mutex_unlock(&g_stw_mutex);
}

static void stw_end(void) {
    pthread_mutex_lock(&g_stw_mutex);
    ny_gc_stw_requested = 0;
    pthread_cond_broadcast(&g_resume);
    // Rejoin the running set, unless this thread is inside a park bracket —
    // a blocking site that called into the allocator — in which case the
    // matching ny_gc_unpark owns that transition.
    if (t_park_depth == 0 && t_state == NY_PARKED) {
        t_state = NY_RUNNING;
        dbg_set_state(NY_RUNNING);
        g_running++;
    }
    dbg_check("stw_end");
    pthread_mutex_unlock(&g_stw_mutex);
}

// ---------------------------------------------------------------------------
// Helpers: object ↔ payload conversion
// ---------------------------------------------------------------------------

static inline NyGcObject *payload_to_header(void *payload) {
    return (NyGcObject *)((uint8_t *)payload - sizeof(NyGcObject));
}

static inline void *header_to_payload(NyGcObject *obj) {
    return (void *)((uint8_t *)obj + sizeof(NyGcObject));
}

// Check if a pointer looks like it points into a GC object's payload.
// Used during conservative scanning of pointer-containing objects.
static int is_gc_pointer(void *ptr) {
    NyGcObject *obj = g_heap.objects;
    while (obj) {
        void *payload = header_to_payload(obj);
        void *end = (uint8_t *)payload + obj->size;
        if (ptr >= payload && ptr < end && ptr == payload) {
            return 1;
        }
        obj = obj->next;
    }
    return 0;
}

// Find the GC header for a given payload pointer (or NULL).
static NyGcObject *find_object(void *payload) {
    if (!payload) return NULL;
    NyGcObject *candidate = payload_to_header(payload);
    // Verify it's in our object list
    NyGcObject *obj = g_heap.objects;
    while (obj) {
        if (obj == candidate) return obj;
        obj = obj->next;
    }
    return NULL;
}

// ---------------------------------------------------------------------------
// Mark phase
// ---------------------------------------------------------------------------

static void mark_object(void *payload);

static void scan_object(NyGcObject *obj) {
    if (!obj->has_pointers) return;

    // Conservative scan: treat every aligned pointer-sized word in the
    // payload as a potential pointer to another GC object.
    void **words = (void **)header_to_payload(obj);
    int64_t num_words = obj->size / (int64_t)sizeof(void *);

    for (int64_t i = 0; i < num_words; i++) {
        void *candidate = words[i];
        if (candidate) {
            mark_object(candidate);
        }
    }
}

static void mark_object(void *payload) {
    if (!payload) return;

    NyGcObject *obj = find_object(payload);
    if (!obj) return;                   // not a GC pointer
    if (obj->mark != NY_GC_WHITE) return; // already visited

    obj->mark = NY_GC_GRAY;
    scan_object(obj);
    obj->mark = NY_GC_BLACK;
}

// Caller must hold g_heap_mutex, which also keeps the thread list stable.
static void mark_roots(void) {
    for (NyGcShadowStack *st = g_heap.thread_roots; st; st = st->next) {
        for (int64_t i = 0; i < st->count; i++) {
            void **slot = (void **)st->entries[i];
            if (slot && *slot) {
                mark_object(*slot);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Sweep phase
// ---------------------------------------------------------------------------

static void sweep(void) {
    NyGcObject **prev = &g_heap.objects;
    NyGcObject *obj = g_heap.objects;

    while (obj) {
        if (obj->mark == NY_GC_WHITE) {
            // Unreachable — free it
            NyGcObject *unreachable = obj;
            *prev = obj->next;
            obj = obj->next;

            int64_t total_size = (int64_t)sizeof(NyGcObject) + unreachable->size;
            g_heap.bytes_allocated -= total_size;
            g_heap.total_freed += total_size;
            free(unreachable);
        } else {
            // Reachable — reset mark for next cycle
            obj->mark = NY_GC_WHITE;
            prev = &obj->next;
            obj = obj->next;
        }
    }
}

// ---------------------------------------------------------------------------
// Collection
// ---------------------------------------------------------------------------

// Caller must have stopped the world: every other participating thread parked.
// That is what makes it safe to touch the heap without holding g_heap_mutex.
static void gc_collect_locked(void) {
    if (!g_initialized) return;

    g_heap.collections++;

    // Mark all reachable objects from every thread's roots
    mark_roots();

    // Sweep unreachable objects
    sweep();

    // Adjust threshold: next collection after 2x current live data
    int64_t live = g_heap.bytes_allocated;
    int64_t new_threshold = live * NY_GC_GROWTH_FACTOR;
    if (new_threshold < NY_GC_DEFAULT_THRESHOLD) {
        new_threshold = NY_GC_DEFAULT_THRESHOLD;
    }
    g_heap.threshold = new_threshold;
}

// Run a collection with every other participating thread parked.
//
// Lock order is g_stw_mutex (inside stw_begin) before g_heap_mutex. Taking
// the heap lock first would deadlock: this thread would hold it while waiting
// for others to park, and a thread parking through ny_gc_safepoint must not
// need that lock to do so — which is why the handshake has its own.
void ny_gc_collect(void) {
    stw_begin();

    // The heap lock is still required. Stopping the world parks every thread
    // running Ny code, but a thread can be inside ny_gc_alloc holding this
    // lock and linking an object — parked threads are not necessarily idle
    // with respect to the heap. Marking without it would race with that.
    pthread_mutex_lock(&g_heap_mutex);
    gc_collect_locked();
    pthread_mutex_unlock(&g_heap_mutex);

    stw_end();
}

// ---------------------------------------------------------------------------
// Allocation
// ---------------------------------------------------------------------------

void *ny_gc_alloc(int64_t size, int8_t has_pointers) {
    // Every wait for the heap lock happens parked — see heap_lock. Blocking on
    // a mutex is not a safepoint, so a thread waiting here while counted as
    // running would stall the handshake until the lock's owner released it.
    heap_lock();
    gc_init_locked();
    int should_collect = g_heap.bytes_allocated + size > g_heap.threshold;
    heap_unlock();

    if (should_collect) {
        ny_gc_collect();
    }

    heap_lock();
    int64_t total = (int64_t)sizeof(NyGcObject) + size;
    NyGcObject *obj = (NyGcObject *)malloc(total);
    if (!obj) {
        // Retry after a collection, again with the lock released.
        heap_unlock();
        ny_gc_collect();
        heap_lock();
        obj = (NyGcObject *)malloc(total);
        if (!obj) {
            heap_unlock();
            fprintf(stderr, "ny: out of memory (gc_alloc %lld bytes)\n",
                    (long long)size);
            abort();
        }
    }

    memset(obj, 0, total);
    obj->mark = NY_GC_WHITE;
    obj->size = (uint32_t)size;
    obj->has_pointers = has_pointers ? 1 : 0;

    // Link into object list
    obj->next = g_heap.objects;
    g_heap.objects = obj;

    g_heap.bytes_allocated += total;

    heap_unlock();
    return header_to_payload(obj);
}

// ---------------------------------------------------------------------------
// Stats / Query
// ---------------------------------------------------------------------------

void ny_gc_stats(void) {
    pthread_mutex_lock(&g_heap_mutex);
    int64_t roots = 0;
    int64_t threads = 0;
    for (NyGcShadowStack *st = g_heap.thread_roots; st; st = st->next) {
        roots += st->count;
        threads++;
    }
    fprintf(stderr,
            "[gc] allocated: %lld bytes | collections: %lld | freed: %lld bytes | roots: %lld (%lld threads)\n",
            (long long)g_heap.bytes_allocated,
            (long long)g_heap.collections,
            (long long)g_heap.total_freed,
            (long long)roots,
            (long long)threads);
    pthread_mutex_unlock(&g_heap_mutex);
}

int64_t ny_gc_bytes_allocated(void) {
    pthread_mutex_lock(&g_heap_mutex);
    int64_t n = g_heap.bytes_allocated;
    pthread_mutex_unlock(&g_heap_mutex);
    return n;
}

int64_t ny_gc_collection_count(void) {
    pthread_mutex_lock(&g_heap_mutex);
    int64_t n = g_heap.collections;
    pthread_mutex_unlock(&g_heap_mutex);
    return n;
}
