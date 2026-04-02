// Ny Lang runtime: fixed-size thread pool + parallel iterators
// Work queue with pthread mutex + condvar

#include <stdlib.h>
#include <stdint.h>
#include <string.h>
#include <pthread.h>

// --- Work Item ---

typedef struct {
    void *(*fn)(void *);
    void *arg;
} WorkItem;

// --- Thread Pool ---

#define MAX_QUEUE 4096

typedef struct {
    pthread_t *threads;
    int num_threads;
    WorkItem queue[MAX_QUEUE];
    int queue_head;
    int queue_tail;
    int queue_count;
    int active_count;
    int shutdown;
    pthread_mutex_t mutex;
    pthread_cond_t work_available;
    pthread_cond_t all_done;
} NyPool;

static void *worker_thread(void *arg) {
    NyPool *pool = (NyPool *)arg;
    while (1) {
        pthread_mutex_lock(&pool->mutex);
        while (pool->queue_count == 0 && !pool->shutdown) {
            pthread_cond_wait(&pool->work_available, &pool->mutex);
        }
        if (pool->shutdown && pool->queue_count == 0) {
            pthread_mutex_unlock(&pool->mutex);
            return NULL;
        }
        // Dequeue work item
        WorkItem item = pool->queue[pool->queue_head];
        pool->queue_head = (pool->queue_head + 1) % MAX_QUEUE;
        pool->queue_count--;
        pool->active_count++;
        pthread_mutex_unlock(&pool->mutex);

        // Execute
        item.fn(item.arg);

        // Done
        pthread_mutex_lock(&pool->mutex);
        pool->active_count--;
        if (pool->queue_count == 0 && pool->active_count == 0) {
            pthread_cond_broadcast(&pool->all_done);
        }
        pthread_mutex_unlock(&pool->mutex);
    }
    return NULL;
}

NyPool *ny_pool_new(int32_t num_threads) {
    if (num_threads <= 0) num_threads = 1;
    NyPool *pool = (NyPool *)calloc(1, sizeof(NyPool));
    pool->threads = (pthread_t *)malloc(num_threads * sizeof(pthread_t));
    pool->num_threads = num_threads;
    pool->shutdown = 0;
    pthread_mutex_init(&pool->mutex, NULL);
    pthread_cond_init(&pool->work_available, NULL);
    pthread_cond_init(&pool->all_done, NULL);

    for (int i = 0; i < num_threads; i++) {
        pthread_create(&pool->threads[i], NULL, worker_thread, pool);
    }
    return pool;
}

void ny_pool_submit(NyPool *pool, void *(*fn)(void *)) {
    pthread_mutex_lock(&pool->mutex);
    if (pool->queue_count < MAX_QUEUE) {
        pool->queue[pool->queue_tail].fn = fn;
        pool->queue[pool->queue_tail].arg = NULL;
        pool->queue_tail = (pool->queue_tail + 1) % MAX_QUEUE;
        pool->queue_count++;
        pthread_cond_signal(&pool->work_available);
    }
    pthread_mutex_unlock(&pool->mutex);
}

void ny_pool_wait(NyPool *pool) {
    pthread_mutex_lock(&pool->mutex);
    while (pool->queue_count > 0 || pool->active_count > 0) {
        pthread_cond_wait(&pool->all_done, &pool->mutex);
    }
    pthread_mutex_unlock(&pool->mutex);
}

void ny_pool_free(NyPool *pool) {
    pthread_mutex_lock(&pool->mutex);
    pool->shutdown = 1;
    pthread_cond_broadcast(&pool->work_available);
    pthread_mutex_unlock(&pool->mutex);

    for (int i = 0; i < pool->num_threads; i++) {
        pthread_join(pool->threads[i], NULL);
    }
    free(pool->threads);
    pthread_mutex_destroy(&pool->mutex);
    pthread_cond_destroy(&pool->work_available);
    pthread_cond_destroy(&pool->all_done);
    free(pool);
}

// --- Parallel Iterators ---

typedef int32_t (*MapFn)(int32_t);
typedef int32_t (*ReduceFn)(int32_t, int32_t);

typedef struct {
    int32_t *data;
    int32_t *result;
    int32_t start;
    int32_t end;
    MapFn fn;
} MapChunk;

typedef struct {
    int32_t *data;
    int32_t start;
    int32_t end;
    int32_t init;
    ReduceFn fn;
    int32_t result;
} ReduceChunk;

static void *map_chunk_worker(void *arg) {
    MapChunk *chunk = (MapChunk *)arg;
    for (int32_t i = chunk->start; i < chunk->end; i++) {
        chunk->result[i] = chunk->fn(chunk->data[i]);
    }
    return NULL;
}

static void *reduce_chunk_worker(void *arg) {
    ReduceChunk *chunk = (ReduceChunk *)arg;
    int32_t acc = chunk->init;
    for (int32_t i = chunk->start; i < chunk->end; i++) {
        acc = chunk->fn(acc, chunk->data[i]);
    }
    chunk->result = acc;
    return NULL;
}

void ny_par_map(int32_t *data, int32_t n, int32_t *result, MapFn fn, NyPool *pool) {
    int num_chunks = pool->num_threads;
    if (num_chunks > n) num_chunks = n;
    int chunk_size = n / num_chunks;

    MapChunk *chunks = (MapChunk *)malloc(num_chunks * sizeof(MapChunk));
    for (int i = 0; i < num_chunks; i++) {
        chunks[i].data = data;
        chunks[i].result = result;
        chunks[i].start = i * chunk_size;
        chunks[i].end = (i == num_chunks - 1) ? n : (i + 1) * chunk_size;
        chunks[i].fn = fn;

        // Submit each chunk as a work item
        pthread_mutex_lock(&pool->mutex);
        if (pool->queue_count < MAX_QUEUE) {
            pool->queue[pool->queue_tail].fn = map_chunk_worker;
            pool->queue[pool->queue_tail].arg = &chunks[i];
            pool->queue_tail = (pool->queue_tail + 1) % MAX_QUEUE;
            pool->queue_count++;
            pthread_cond_signal(&pool->work_available);
        }
        pthread_mutex_unlock(&pool->mutex);
    }

    ny_pool_wait(pool);
    free(chunks);
}

int32_t ny_par_reduce(int32_t *data, int32_t n, int32_t init, ReduceFn fn, NyPool *pool) {
    int num_chunks = pool->num_threads;
    if (num_chunks > n) num_chunks = n;
    int chunk_size = n / num_chunks;

    ReduceChunk *chunks = (ReduceChunk *)malloc(num_chunks * sizeof(ReduceChunk));
    for (int i = 0; i < num_chunks; i++) {
        chunks[i].data = data;
        chunks[i].start = i * chunk_size;
        chunks[i].end = (i == num_chunks - 1) ? n : (i + 1) * chunk_size;
        chunks[i].init = (i == 0) ? init : 0;
        chunks[i].fn = fn;
        chunks[i].result = 0;

        pthread_mutex_lock(&pool->mutex);
        if (pool->queue_count < MAX_QUEUE) {
            pool->queue[pool->queue_tail].fn = reduce_chunk_worker;
            pool->queue[pool->queue_tail].arg = &chunks[i];
            pool->queue_tail = (pool->queue_tail + 1) % MAX_QUEUE;
            pool->queue_count++;
            pthread_cond_signal(&pool->work_available);
        }
        pthread_mutex_unlock(&pool->mutex);
    }

    ny_pool_wait(pool);

    // Combine partial results
    int32_t total = init;
    for (int i = 0; i < num_chunks; i++) {
        total = fn(total, chunks[i].result);
    }
    free(chunks);
    return total;
}
