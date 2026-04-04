// Ny Lang runtime: bounded channel for thread-safe message passing
// Ring buffer with pthread mutex + condvar for blocking send/recv

#include <stdlib.h>
#include <stdint.h>
#include <pthread.h>
#include <time.h>

typedef struct {
    int32_t *buffer;
    int head;
    int tail;
    int count;
    int capacity;
    int closed;
    pthread_mutex_t mutex;
    pthread_cond_t not_empty;
    pthread_cond_t not_full;
} NyChannel;

NyChannel *ny_channel_new(int32_t capacity) {
    if (capacity <= 0) capacity = 16;
    NyChannel *ch = (NyChannel *)malloc(sizeof(NyChannel));
    ch->buffer = (int32_t *)malloc(capacity * sizeof(int32_t));
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

void ny_channel_send(NyChannel *ch, int32_t value) {
    pthread_mutex_lock(&ch->mutex);
    while (ch->count == ch->capacity && !ch->closed) {
        pthread_cond_wait(&ch->not_full, &ch->mutex);
    }
    if (ch->closed) {
        pthread_mutex_unlock(&ch->mutex);
        return;
    }
    ch->buffer[ch->tail] = value;
    ch->tail = (ch->tail + 1) % ch->capacity;
    ch->count++;
    pthread_cond_signal(&ch->not_empty);
    pthread_mutex_unlock(&ch->mutex);
}

int32_t ny_channel_recv(NyChannel *ch) {
    pthread_mutex_lock(&ch->mutex);
    while (ch->count == 0 && !ch->closed) {
        pthread_cond_wait(&ch->not_empty, &ch->mutex);
    }
    if (ch->count == 0 && ch->closed) {
        pthread_mutex_unlock(&ch->mutex);
        return 0; // sentinel for closed channel
    }
    int32_t value = ch->buffer[ch->head];
    ch->head = (ch->head + 1) % ch->capacity;
    ch->count--;
    pthread_cond_signal(&ch->not_full);
    pthread_mutex_unlock(&ch->mutex);
    return value;
}

void ny_channel_close(NyChannel *ch) {
    pthread_mutex_lock(&ch->mutex);
    ch->closed = 1;
    pthread_cond_broadcast(&ch->not_empty);
    pthread_cond_broadcast(&ch->not_full);
    pthread_mutex_unlock(&ch->mutex);
}

// Non-blocking try_recv: returns 1 if a value was received, 0 otherwise.
// On success, *out_value is set to the received value.
int32_t ny_channel_try_recv(NyChannel *ch, int32_t *out_value) {
    pthread_mutex_lock(&ch->mutex);
    if (ch->count == 0) {
        pthread_mutex_unlock(&ch->mutex);
        *out_value = 0;
        return 0;
    }
    *out_value = ch->buffer[ch->head];
    ch->head = (ch->head + 1) % ch->capacity;
    ch->count--;
    pthread_cond_signal(&ch->not_full);
    pthread_mutex_unlock(&ch->mutex);
    return 1;
}

// Select over multiple channels for recv.
// channels: array of NyChannel* pointers
// n: number of channels
// out_value: receives the value from the ready channel
// Returns the index (0-based) of the channel that was ready, or -1 on timeout.
// Polls in round-robin; blocks briefly between rounds.
int32_t ny_channel_select(NyChannel **channels, int32_t n, int32_t *out_value) {
    // Spin-poll with short sleeps
    struct timespec ts = {0, 1000000}; // 1ms
    for (int attempt = 0; attempt < 10000; attempt++) {
        for (int32_t i = 0; i < n; i++) {
            if (ny_channel_try_recv(channels[i], out_value)) {
                return i;
            }
        }
        nanosleep(&ts, NULL);
    }
    *out_value = 0;
    return -1; // timeout after ~10 seconds
}
