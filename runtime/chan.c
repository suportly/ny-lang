// Ny Lang runtime: generic typed channel for thread-safe message passing
// Ring buffer with pthread mutex + condvar for blocking send/recv.
// Supports arbitrary element sizes (i32, f64, str, structs, pointers).
//
// Usage from Ny code:
//   ch : chan<i32> = chan_new(16);
//   ch.send(42);
//   val := ch.recv();
//   ch.close();

#include <stdlib.h>
#include <stdint.h>
#include <string.h>
#include <pthread.h>

typedef struct {
    uint8_t *buffer;        // ring buffer (elem_size * capacity bytes)
    int64_t elem_size;      // size of each element in bytes
    int32_t head;
    int32_t tail;
    int32_t count;
    int32_t capacity;
    int32_t closed;
    pthread_mutex_t mutex;
    pthread_cond_t not_empty;
    pthread_cond_t not_full;
} NyChan;

// Create a new channel with given capacity and element size.
NyChan *ny_chan_new(int32_t capacity, int64_t elem_size) {
    if (capacity <= 0) capacity = 16;
    if (elem_size <= 0) elem_size = 8;
    NyChan *ch = (NyChan *)malloc(sizeof(NyChan));
    if (!ch) return NULL;
    ch->buffer = (uint8_t *)calloc(capacity, elem_size);
    if (!ch->buffer) { free(ch); return NULL; }
    ch->elem_size = elem_size;
    ch->head = 0;
    ch->tail = 0;
    ch->count = 0;
    ch->capacity = capacity;
    ch->closed = 0;
    pthread_mutex_init(&ch->mutex, NULL);
    pthread_cond_init(&ch->not_empty, NULL);
    pthread_cond_init(&ch->not_full, NULL);
    return ch;
}

// Send a value into the channel. Blocks if full.
// `value_ptr` points to the data to copy (elem_size bytes).
void ny_chan_send(NyChan *ch, const void *value_ptr) {
    pthread_mutex_lock(&ch->mutex);
    while (ch->count == ch->capacity && !ch->closed) {
        pthread_cond_wait(&ch->not_full, &ch->mutex);
    }
    if (ch->closed) {
        pthread_mutex_unlock(&ch->mutex);
        return;
    }
    memcpy(ch->buffer + (int64_t)ch->tail * ch->elem_size, value_ptr, ch->elem_size);
    ch->tail = (ch->tail + 1) % ch->capacity;
    ch->count++;
    pthread_cond_signal(&ch->not_empty);
    pthread_mutex_unlock(&ch->mutex);
}

// Receive a value from the channel. Blocks if empty.
// Copies elem_size bytes into `out_ptr`.
void ny_chan_recv(NyChan *ch, void *out_ptr) {
    pthread_mutex_lock(&ch->mutex);
    while (ch->count == 0 && !ch->closed) {
        pthread_cond_wait(&ch->not_empty, &ch->mutex);
    }
    if (ch->count == 0 && ch->closed) {
        memset(out_ptr, 0, ch->elem_size);
        pthread_mutex_unlock(&ch->mutex);
        return;
    }
    memcpy(out_ptr, ch->buffer + (int64_t)ch->head * ch->elem_size, ch->elem_size);
    ch->head = (ch->head + 1) % ch->capacity;
    ch->count--;
    pthread_cond_signal(&ch->not_full);
    pthread_mutex_unlock(&ch->mutex);
}

// Close the channel. Wakes all waiting senders/receivers.
void ny_chan_close(NyChan *ch) {
    pthread_mutex_lock(&ch->mutex);
    ch->closed = 1;
    pthread_cond_broadcast(&ch->not_empty);
    pthread_cond_broadcast(&ch->not_full);
    pthread_mutex_unlock(&ch->mutex);
}

// Non-blocking try_recv. Returns 1 if a value was received, 0 otherwise.
int32_t ny_chan_try_recv(NyChan *ch, void *out_ptr) {
    pthread_mutex_lock(&ch->mutex);
    if (ch->count == 0) {
        pthread_mutex_unlock(&ch->mutex);
        return 0;
    }
    memcpy(out_ptr, ch->buffer + (int64_t)ch->head * ch->elem_size, ch->elem_size);
    ch->head = (ch->head + 1) % ch->capacity;
    ch->count--;
    pthread_cond_signal(&ch->not_full);
    pthread_mutex_unlock(&ch->mutex);
    return 1;
}

// Free the channel and its buffer.
void ny_chan_free(NyChan *ch) {
    if (!ch) return;
    pthread_mutex_destroy(&ch->mutex);
    pthread_cond_destroy(&ch->not_empty);
    pthread_cond_destroy(&ch->not_full);
    free(ch->buffer);
    free(ch);
}
