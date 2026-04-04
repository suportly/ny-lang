// Ny Lang runtime: async/await — Future<T> backed by thread pool

#include <stdlib.h>
#include <stdint.h>
#include <pthread.h>
#include <unistd.h>

// Forward declare from threadpool.c
typedef struct NyPool NyPool;
extern NyPool *ny_pool_new(int32_t num_threads);
extern void ny_pool_submit_arg(NyPool *pool, void *(*fn)(void *), void *arg);
extern void ny_pool_wait(NyPool *pool);
extern void ny_pool_free(NyPool *pool);

// --- Global async pool (lazy init) ---

static NyPool *ny_global_async_pool = NULL;
static pthread_once_t ny_async_once = PTHREAD_ONCE_INIT;
static int ny_async_num_threads = 0;

static void ny_async_init_internal(void) {
    int n = ny_async_num_threads > 0 ? ny_async_num_threads : (int)sysconf(_SC_NPROCESSORS_ONLN);
    if (n < 1) n = 4;
    ny_global_async_pool = ny_pool_new(n);
}

void ny_async_init(int32_t num_threads) {
    ny_async_num_threads = num_threads;
    pthread_once(&ny_async_once, ny_async_init_internal);
}

NyPool *ny_async_pool(void) {
    pthread_once(&ny_async_once, ny_async_init_internal);
    return ny_global_async_pool;
}

// Wait for all goroutines, then free the pool. Safe if pool was never created.
void ny_async_pool_shutdown(void) {
    if (ny_global_async_pool) {
        ny_pool_wait(ny_global_async_pool);
        ny_pool_free(ny_global_async_pool);
        ny_global_async_pool = NULL;
    }
}

// --- Future ---

typedef struct {
    int64_t result;          // stores the i64 result (covers i32, f64 via bitcast, ptr)
    int done;
    pthread_mutex_t mutex;
    pthread_cond_t cond;
} NyFuture;

NyFuture *ny_future_create(void) {
    NyFuture *f = (NyFuture *)calloc(1, sizeof(NyFuture));
    pthread_mutex_init(&f->mutex, NULL);
    pthread_cond_init(&f->cond, NULL);
    return f;
}

void ny_future_signal(NyFuture *f, int64_t result) {
    pthread_mutex_lock(&f->mutex);
    f->result = result;
    f->done = 1;
    pthread_cond_signal(&f->cond);
    pthread_mutex_unlock(&f->mutex);
}

int64_t ny_future_await(NyFuture *f) {
    pthread_mutex_lock(&f->mutex);
    while (!f->done) {
        pthread_cond_wait(&f->cond, &f->mutex);
    }
    int64_t result = f->result;
    pthread_mutex_unlock(&f->mutex);
    return result;
}

void ny_future_free(NyFuture *f) {
    if (f) {
        pthread_mutex_destroy(&f->mutex);
        pthread_cond_destroy(&f->cond);
        free(f);
    }
}
