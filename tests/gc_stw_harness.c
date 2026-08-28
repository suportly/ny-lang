// Reproduces the runtime's threading pattern against the real GC:
// a main thread that roots an object then blocks on a channel receive,
// pool-style workers that root, allocate hard enough to force collections,
// and idle waits — the exact shape that deadlocked in CI.
#include <stdio.h>
#include <stdlib.h>
#include <pthread.h>
#include "gc.h"

extern void ny_chan_send(void *ch, const void *v);
extern void ny_chan_recv(void *ch, void *out);
extern void *ny_chan_new(int64_t cap, int64_t elem_size);

typedef struct { int a,b,c,d,e,f,g,h; } Cell;

static void *ch;

static void *worker(void *arg) {
    long seed = (long)arg;
    Cell *mine = (Cell *)ny_gc_alloc(sizeof(Cell), 1);
    ny_gc_root_push((void **)&mine);
    mine->a = (int)seed;

    for (int i = 0; i < 40000; i++) {
        Cell *junk = (Cell *)ny_gc_alloc(sizeof(Cell), 1);
        junk->a = i;
        // Simulate a loop back-edge safepoint poll.
        if (ny_gc_stw_requested) ny_gc_safepoint();
    }

    int v = mine->a;
    ny_chan_send(ch, &v);
    ny_gc_root_pop(1);
    return NULL;
}

// Spins with no allocation: only a back-edge poll can park it.
static void *spinner(void *arg) {
    long seed = (long)arg;
    Cell *mine = (Cell *)ny_gc_alloc(sizeof(Cell), 1);
    ny_gc_root_push((void **)&mine);
    mine->a = (int)seed;

    volatile long acc = 0;
    for (long i = 0; i < 20000000L; i++) {
        acc += i % 7;
        if (ny_gc_stw_requested) ny_gc_safepoint();
    }

    int v = mine->a;
    ny_chan_send(ch, &v);
    ny_gc_root_pop(1);
    return NULL;
}

int main(void) {
    ch = ny_chan_new(16, sizeof(int));

    Cell *anchor = (Cell *)ny_gc_alloc(sizeof(Cell), 1);
    ny_gc_root_push((void **)&anchor);
    anchor->a = 6;

    pthread_t t[4];
    pthread_create(&t[0], NULL, worker,  (void *)15L);
    pthread_create(&t[1], NULL, worker,  (void *)5L);
    pthread_create(&t[2], NULL, spinner, (void *)10L);
    pthread_create(&t[3], NULL, spinner, (void *)11L);

    int sum = 0;
    for (int i = 0; i < 4; i++) { int v; ny_chan_recv(ch, &v); sum += v; }
    for (int i = 0; i < 4; i++) pthread_join(t[i], NULL);

    if (sum != 41)      { printf("FAIL sum=%d want 41\n", sum); return 1; }
    if (anchor->a != 6) { printf("FAIL anchor=%d want 6\n", anchor->a); return 2; }
    if (ny_gc_collection_count() < 1) { printf("FAIL no collections\n"); return 3; }

    printf("OK sum=%d anchor=%d collections=%lld\n",
           sum, anchor->a, (long long)ny_gc_collection_count());
    return 0;
}
