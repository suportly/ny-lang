// Ny Lang runtime: minimal string->int64 hashmap
// Compiled and linked with ny programs that use map_* builtins

#include <stdlib.h>
#include <string.h>
#include <stdint.h>

#define INIT_CAP 16
#define LOAD_FACTOR 0.75

typedef struct {
    char *key;
    int64_t key_len;
    int64_t value;
    int occupied;
} Entry;

typedef struct {
    Entry *entries;
    int64_t len;
    int64_t cap;
} NyHashMap;

static uint64_t hash_bytes(const char *data, int64_t len) {
    uint64_t h = 5381;
    for (int64_t i = 0; i < len; i++) {
        h = ((h << 5) + h) + (unsigned char)data[i];
    }
    return h;
}

NyHashMap *ny_map_new(void) {
    NyHashMap *m = (NyHashMap *)malloc(sizeof(NyHashMap));
    m->entries = (Entry *)calloc(INIT_CAP, sizeof(Entry));
    m->len = 0;
    m->cap = INIT_CAP;
    return m;
}

static void map_grow(NyHashMap *m) {
    int64_t new_cap = m->cap * 2;
    Entry *new_entries = (Entry *)calloc(new_cap, sizeof(Entry));

    for (int64_t i = 0; i < m->cap; i++) {
        if (m->entries[i].occupied) {
            uint64_t h = hash_bytes(m->entries[i].key, m->entries[i].key_len) % new_cap;
            while (new_entries[h].occupied) {
                h = (h + 1) % new_cap;
            }
            new_entries[h] = m->entries[i];
        }
    }

    free(m->entries);
    m->entries = new_entries;
    m->cap = new_cap;
}

void ny_map_insert(NyHashMap *m, const char *key, int64_t key_len, int64_t value) {
    if ((double)m->len / m->cap >= LOAD_FACTOR) {
        map_grow(m);
    }

    uint64_t h = hash_bytes(key, key_len) % m->cap;
    while (m->entries[h].occupied) {
        if (m->entries[h].key_len == key_len &&
            memcmp(m->entries[h].key, key, key_len) == 0) {
            // Update existing key
            m->entries[h].value = value;
            return;
        }
        h = (h + 1) % m->cap;
    }

    // Insert new entry
    m->entries[h].key = (char *)malloc(key_len);
    memcpy(m->entries[h].key, key, key_len);
    m->entries[h].key_len = key_len;
    m->entries[h].value = value;
    m->entries[h].occupied = 1;
    m->len++;
}

int64_t ny_map_get(NyHashMap *m, const char *key, int64_t key_len) {
    uint64_t h = hash_bytes(key, key_len) % m->cap;
    int64_t probes = 0;
    while (m->entries[h].occupied && probes < m->cap) {
        if (m->entries[h].key_len == key_len &&
            memcmp(m->entries[h].key, key, key_len) == 0) {
            return m->entries[h].value;
        }
        h = (h + 1) % m->cap;
        probes++;
    }
    return 0; // default for missing key
}

int ny_map_contains(NyHashMap *m, const char *key, int64_t key_len) {
    uint64_t h = hash_bytes(key, key_len) % m->cap;
    int64_t probes = 0;
    while (m->entries[h].occupied && probes < m->cap) {
        if (m->entries[h].key_len == key_len &&
            memcmp(m->entries[h].key, key, key_len) == 0) {
            return 1;
        }
        h = (h + 1) % m->cap;
        probes++;
    }
    return 0;
}

int64_t ny_map_len(NyHashMap *m) {
    return m->len;
}
