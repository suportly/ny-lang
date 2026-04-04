// Ny Lang runtime: tracing mark-and-sweep garbage collector
//
// A simple, correct, stop-the-world mark-and-sweep collector with a shadow
// stack for precise root enumeration.
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

// ---------------------------------------------------------------------------
// Global heap (single-threaded for now; will add per-thread heaps later)
// ---------------------------------------------------------------------------

static NyGcHeap g_heap;
static int g_initialized = 0;

// Default threshold: trigger collection after 1MB of allocations
#define NY_GC_DEFAULT_THRESHOLD (1024 * 1024)

// Growth factor: after collection, set threshold to 2x live data
#define NY_GC_GROWTH_FACTOR 2

// ---------------------------------------------------------------------------
// Init / Shutdown
// ---------------------------------------------------------------------------

void ny_gc_init(void) {
    if (g_initialized) return;
    g_heap.objects = NULL;
    g_heap.bytes_allocated = 0;
    g_heap.threshold = NY_GC_DEFAULT_THRESHOLD;
    g_heap.collections = 0;
    g_heap.total_freed = 0;

    g_heap.roots.capacity = NY_GC_SHADOW_STACK_MAX;
    g_heap.roots.entries = (void **)calloc(NY_GC_SHADOW_STACK_MAX, sizeof(void *));
    g_heap.roots.count = 0;

    g_initialized = 1;
}

void ny_gc_shutdown(void) {
    if (!g_initialized) return;

    // Free all remaining objects
    NyGcObject *obj = g_heap.objects;
    while (obj) {
        NyGcObject *next = obj->next;
        free(obj);
        obj = next;
    }

    free(g_heap.roots.entries);
    memset(&g_heap, 0, sizeof(g_heap));
    g_initialized = 0;
}

// ---------------------------------------------------------------------------
// Shadow stack operations
// ---------------------------------------------------------------------------

void ny_gc_root_push(void **slot) {
    if (!g_initialized) ny_gc_init();

    if (g_heap.roots.count >= g_heap.roots.capacity) {
        // Grow shadow stack
        int64_t new_cap = g_heap.roots.capacity * 2;
        void **new_entries = (void **)realloc(
            g_heap.roots.entries, new_cap * sizeof(void *)
        );
        if (!new_entries) {
            fprintf(stderr, "ny: GC shadow stack overflow (%lld roots)\n",
                    (long long)g_heap.roots.count);
            abort();
        }
        g_heap.roots.entries = new_entries;
        g_heap.roots.capacity = new_cap;
    }

    g_heap.roots.entries[g_heap.roots.count++] = slot;
}

void ny_gc_root_pop(int64_t n) {
    g_heap.roots.count -= n;
    if (g_heap.roots.count < 0) g_heap.roots.count = 0;
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

static void mark_roots(void) {
    for (int64_t i = 0; i < g_heap.roots.count; i++) {
        void **slot = (void **)g_heap.roots.entries[i];
        if (slot && *slot) {
            mark_object(*slot);
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

void ny_gc_collect(void) {
    if (!g_initialized) return;

    g_heap.collections++;

    // Mark all reachable objects from roots
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

// ---------------------------------------------------------------------------
// Allocation
// ---------------------------------------------------------------------------

void *ny_gc_alloc(int64_t size, int8_t has_pointers) {
    if (!g_initialized) ny_gc_init();

    // Check if we should collect before allocating
    if (g_heap.bytes_allocated + size > g_heap.threshold) {
        ny_gc_collect();
    }

    int64_t total = (int64_t)sizeof(NyGcObject) + size;
    NyGcObject *obj = (NyGcObject *)malloc(total);
    if (!obj) {
        // Try collecting and retrying
        ny_gc_collect();
        obj = (NyGcObject *)malloc(total);
        if (!obj) {
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

    return header_to_payload(obj);
}

// ---------------------------------------------------------------------------
// Stats / Query
// ---------------------------------------------------------------------------

void ny_gc_stats(void) {
    fprintf(stderr, "[gc] allocated: %lld bytes | collections: %lld | freed: %lld bytes | roots: %lld\n",
            (long long)g_heap.bytes_allocated,
            (long long)g_heap.collections,
            (long long)g_heap.total_freed,
            (long long)g_heap.roots.count);
}

int64_t ny_gc_bytes_allocated(void) {
    return g_heap.bytes_allocated;
}

int64_t ny_gc_collection_count(void) {
    return g_heap.collections;
}
