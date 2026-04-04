// Ny Lang runtime: tracing mark-and-sweep garbage collector
//
// Design: conservative, stop-the-world, mark-and-sweep with shadow stack.
// Inspired by Go's original GC — simple, correct, predictable.
//
// Usage from Ny code:
//   p := gc_alloc(sizeof(Point));   // GC-managed allocation
//   // no free needed — collector reclaims when unreachable
//   gc_collect();                   // explicit collection (optional)
//   gc_stats();                     // print heap stats

#ifndef NY_GC_H
#define NY_GC_H

#include <stdint.h>
#include <stddef.h>

// ---------------------------------------------------------------------------
// Object header — prepended to every GC-managed allocation
// ---------------------------------------------------------------------------

#define NY_GC_WHITE 0   // not yet visited
#define NY_GC_GRAY  1   // reachable, children not yet scanned
#define NY_GC_BLACK 2   // reachable, all children scanned

typedef struct NyGcObject {
    struct NyGcObject *next;    // intrusive linked list of all objects
    uint32_t mark;              // WHITE / GRAY / BLACK
    uint32_t size;              // payload size in bytes
    uint8_t  has_pointers;      // 1 if payload may contain pointers
    uint8_t  _pad[3];
} NyGcObject;

// ---------------------------------------------------------------------------
// Shadow stack — root registration for precise GC roots
// ---------------------------------------------------------------------------

#define NY_GC_SHADOW_STACK_MAX 4096

typedef struct {
    void **entries;             // array of pointers to GC-managed objects
    int64_t count;
    int64_t capacity;
} NyGcShadowStack;

// ---------------------------------------------------------------------------
// Heap state
// ---------------------------------------------------------------------------

typedef struct {
    NyGcObject *objects;        // linked list of all live objects
    int64_t bytes_allocated;    // total bytes currently allocated
    int64_t threshold;          // collect when bytes_allocated > threshold
    int64_t collections;        // number of collections performed
    int64_t total_freed;        // cumulative bytes freed
    NyGcShadowStack roots;     // shadow stack for GC roots
} NyGcHeap;

// ---------------------------------------------------------------------------
// Public API (called from generated code)
// ---------------------------------------------------------------------------

// Initialize the global GC heap. Called once at program start.
void ny_gc_init(void);

// Allocate `size` bytes of GC-managed memory.
// `has_pointers`: 1 if the allocated block may contain pointers to other
// GC-managed objects (structs, arrays of pointers). 0 for leaf data
// (integers, floats, raw byte buffers).
void *ny_gc_alloc(int64_t size, int8_t has_pointers);

// Run a full mark-and-sweep collection.
void ny_gc_collect(void);

// Push a root pointer onto the shadow stack (called at function entry
// or when a local variable receives a GC pointer).
void ny_gc_root_push(void **slot);

// Pop `n` roots from the shadow stack (called at function exit).
void ny_gc_root_pop(int64_t n);

// Print GC statistics to stderr.
void ny_gc_stats(void);

// Shutdown: free all objects and the heap itself.
void ny_gc_shutdown(void);

// Query functions
int64_t ny_gc_bytes_allocated(void);
int64_t ny_gc_collection_count(void);

#endif // NY_GC_H
