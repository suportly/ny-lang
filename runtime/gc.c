// Ny Lang runtime: tracing mark-and-sweep garbage collector
//
// A mark-and-sweep collector with per-thread shadow stacks for precise root
// enumeration. It is NOT stop-the-world: a mutex serialises allocation and
// collection so the heap structures stay consistent, but mutator threads keep
// running while a collection marks. See gc_collect_locked for what that costs
// and what closing the gap would take.
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

// Per-thread shadow stack. Zero-initialised, so a thread that never roots
// anything costs nothing and is never registered.
static __thread NyGcShadowStack t_roots;

// Key whose only job is to run a destructor when a thread exits, so its
// shadow stack is unlinked before the thread's storage goes away.
static pthread_key_t g_thread_exit_key;
static pthread_once_t g_thread_exit_once = PTHREAD_ONCE_INIT;

static void unregister_thread_roots(void *unused);

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

    pthread_mutex_lock(&g_heap_mutex);
    gc_init_locked();
    t_roots.next = g_heap.thread_roots;
    g_heap.thread_roots = &t_roots;
    t_roots.registered = 1;
    pthread_mutex_unlock(&g_heap_mutex);
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

        pthread_mutex_lock(&g_heap_mutex);
        void **old_entries = t_roots.entries;
        t_roots.entries = new_entries;
        t_roots.capacity = new_cap;
        pthread_mutex_unlock(&g_heap_mutex);

        free(old_entries);
    }

    t_roots.entries[t_roots.count++] = slot;
}

void ny_gc_root_pop(int64_t n) {
    t_roots.count -= n;
    if (t_roots.count < 0) t_roots.count = 0;
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

// Caller must hold g_heap_mutex.
//
// NOT stop-the-world. The lock keeps the heap structures consistent — no two
// threads splice the object list or sweep at once — but other threads keep
// running Ny code while this marks. A thread that stores a freshly allocated
// pointer into an already-marked object during the mark can have that object
// swept, because nothing re-scans it.
//
// Closing that window needs safepoints: the codegen has to emit polls that
// let every thread be parked at a known point before marking starts. Until
// then, `go` + `new` in the same program carries this risk. See
// docs/LIMITATIONS.md.
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

void ny_gc_collect(void) {
    pthread_mutex_lock(&g_heap_mutex);
    gc_collect_locked();
    pthread_mutex_unlock(&g_heap_mutex);
}

// ---------------------------------------------------------------------------
// Allocation
// ---------------------------------------------------------------------------

void *ny_gc_alloc(int64_t size, int8_t has_pointers) {
    pthread_mutex_lock(&g_heap_mutex);
    gc_init_locked();

    // Check if we should collect before allocating
    if (g_heap.bytes_allocated + size > g_heap.threshold) {
        gc_collect_locked();
    }

    int64_t total = (int64_t)sizeof(NyGcObject) + size;
    NyGcObject *obj = (NyGcObject *)malloc(total);
    if (!obj) {
        // Try collecting and retrying
        gc_collect_locked();
        obj = (NyGcObject *)malloc(total);
        if (!obj) {
            pthread_mutex_unlock(&g_heap_mutex);
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

    pthread_mutex_unlock(&g_heap_mutex);
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
