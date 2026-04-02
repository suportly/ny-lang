// Ny Lang runtime: arena (bump) allocator
// Perfect for ML workloads: allocate many objects, free all at once.
//
// Usage:
//   arena := arena_new(1024 * 1024);  // 1MB initial chunk
//   defer arena_free(arena);
//   buf := arena_alloc(arena, 256);   // bump-allocate 256 bytes
//   // ... use buf ...
//   // arena_free releases ALL allocations at once

#include <stdlib.h>
#include <stdint.h>
#include <string.h>

#define DEFAULT_CHUNK_SIZE (64 * 1024) // 64KB default

typedef struct Chunk {
    uint8_t *data;
    int64_t size;
    int64_t used;
    struct Chunk *next;
} Chunk;

typedef struct {
    Chunk *current;
    Chunk *first;
    int64_t default_chunk_size;
    int64_t total_allocated;
} NyArena;

static Chunk *chunk_new(int64_t size) {
    Chunk *c = (Chunk *)malloc(sizeof(Chunk));
    if (!c) return NULL;
    c->data = (uint8_t *)malloc(size);
    if (!c->data) { free(c); return NULL; }
    c->size = size;
    c->used = 0;
    c->next = NULL;
    return c;
}

NyArena *ny_arena_new(int64_t size_hint) {
    NyArena *a = (NyArena *)malloc(sizeof(NyArena));
    if (!a) return NULL;
    int64_t chunk_size = size_hint > 0 ? size_hint : DEFAULT_CHUNK_SIZE;
    a->first = chunk_new(chunk_size);
    a->current = a->first;
    a->default_chunk_size = chunk_size;
    a->total_allocated = 0;
    return a;
}

void *ny_arena_alloc(NyArena *a, int64_t size) {
    if (!a || !a->current) return NULL;

    // Align to 8 bytes
    int64_t aligned_size = (size + 7) & ~7;

    // Try current chunk
    if (a->current->used + aligned_size <= a->current->size) {
        void *ptr = a->current->data + a->current->used;
        a->current->used += aligned_size;
        a->total_allocated += aligned_size;
        return ptr;
    }

    // Need new chunk — at least aligned_size or default, whichever is bigger
    int64_t new_chunk_size = a->default_chunk_size;
    if (aligned_size > new_chunk_size) {
        new_chunk_size = aligned_size;
    }

    Chunk *new_chunk = chunk_new(new_chunk_size);
    if (!new_chunk) return NULL;

    // Link into chain
    a->current->next = new_chunk;
    a->current = new_chunk;

    void *ptr = new_chunk->data;
    new_chunk->used = aligned_size;
    a->total_allocated += aligned_size;
    return ptr;
}

void ny_arena_free(NyArena *a) {
    if (!a) return;
    Chunk *c = a->first;
    while (c) {
        Chunk *next = c->next;
        free(c->data);
        free(c);
        c = next;
    }
    free(a);
}

void ny_arena_reset(NyArena *a) {
    if (!a) return;
    // Free all chunks except the first, reset first
    Chunk *c = a->first->next;
    while (c) {
        Chunk *next = c->next;
        free(c->data);
        free(c);
        c = next;
    }
    a->first->next = NULL;
    a->first->used = 0;
    a->current = a->first;
    a->total_allocated = 0;
}

int64_t ny_arena_bytes_used(NyArena *a) {
    return a ? a->total_allocated : 0;
}
