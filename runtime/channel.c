// Ny Lang runtime: bounded channel for thread-safe message passing
// Ring buffer with pthread mutex + condvar for blocking send/recv

#include <stdlib.h>
#include <stdint.h>
#include <pthread.h>

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
