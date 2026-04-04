// Ny Lang runtime: generic HashMap<K,V>
// Keys are always {ptr, len} strings. Values are arbitrary bytes (memcpy'd).

#include <stdlib.h>
#include <string.h>
#include <stdint.h>

#define HMAP_INIT_CAP 16
#define HMAP_LOAD_FACTOR 0.75

typedef struct {
    char *key;
    int64_t key_len;
    char *value;  // malloc'd buffer of val_size bytes
    int occupied;
} HEntry;

typedef struct {
    HEntry *entries;
    int64_t len;
    int64_t cap;
    int64_t val_size;  // sizeof(V)
} NyHMap;

static uint64_t hmap_hash(const char *data, int64_t len) {
    uint64_t h = 5381;
    for (int64_t i = 0; i < len; i++)
        h = ((h << 5) + h) + (unsigned char)data[i];
    return h;
}

NyHMap *ny_hmap_new(int64_t val_size) {
    NyHMap *m = (NyHMap *)malloc(sizeof(NyHMap));
    m->entries = (HEntry *)calloc(HMAP_INIT_CAP, sizeof(HEntry));
    m->len = 0;
    m->cap = HMAP_INIT_CAP;
    m->val_size = val_size;
    return m;
}

static void hmap_grow(NyHMap *m) {
    int64_t new_cap = m->cap * 2;
    HEntry *new_entries = (HEntry *)calloc(new_cap, sizeof(HEntry));
    for (int64_t i = 0; i < m->cap; i++) {
        if (m->entries[i].occupied) {
            uint64_t h = hmap_hash(m->entries[i].key, m->entries[i].key_len) % new_cap;
            while (new_entries[h].occupied) h = (h + 1) % new_cap;
            new_entries[h] = m->entries[i];
        }
    }
    free(m->entries);
    m->entries = new_entries;
    m->cap = new_cap;
}

void ny_hmap_insert(NyHMap *m, const char *key, int64_t key_len, const void *value) {
    if ((double)m->len / m->cap >= HMAP_LOAD_FACTOR) hmap_grow(m);
    uint64_t h = hmap_hash(key, key_len) % m->cap;
    while (m->entries[h].occupied) {
        if (m->entries[h].key_len == key_len && memcmp(m->entries[h].key, key, key_len) == 0) {
            memcpy(m->entries[h].value, value, m->val_size);
            return;
        }
        h = (h + 1) % m->cap;
    }
    m->entries[h].key = (char *)malloc(key_len);
    memcpy(m->entries[h].key, key, key_len);
    m->entries[h].key_len = key_len;
    m->entries[h].value = (char *)malloc(m->val_size);
    memcpy(m->entries[h].value, value, m->val_size);
    m->entries[h].occupied = 1;
    m->len++;
}

// Returns 1 if found (copies value to out), 0 if not found
int ny_hmap_get(NyHMap *m, const char *key, int64_t key_len, void *out) {
    uint64_t h = hmap_hash(key, key_len) % m->cap;
    int64_t probes = 0;
    while (m->entries[h].occupied && probes < m->cap) {
        if (m->entries[h].key_len == key_len && memcmp(m->entries[h].key, key, key_len) == 0) {
            memcpy(out, m->entries[h].value, m->val_size);
            return 1;
        }
        h = (h + 1) % m->cap;
        probes++;
    }
    memset(out, 0, m->val_size);
    return 0;
}

int ny_hmap_contains(NyHMap *m, const char *key, int64_t key_len) {
    uint64_t h = hmap_hash(key, key_len) % m->cap;
    int64_t probes = 0;
    while (m->entries[h].occupied && probes < m->cap) {
        if (m->entries[h].key_len == key_len && memcmp(m->entries[h].key, key, key_len) == 0)
            return 1;
        h = (h + 1) % m->cap;
        probes++;
    }
    return 0;
}

void ny_hmap_remove(NyHMap *m, const char *key, int64_t key_len) {
    uint64_t h = hmap_hash(key, key_len) % m->cap;
    int64_t probes = 0;
    while (m->entries[h].occupied && probes < m->cap) {
        if (m->entries[h].key_len == key_len && memcmp(m->entries[h].key, key, key_len) == 0) {
            free(m->entries[h].key);
            free(m->entries[h].value);
            m->entries[h].occupied = 0;
            m->len--;
            return;
        }
        h = (h + 1) % m->cap;
        probes++;
    }
}

int64_t ny_hmap_len(NyHMap *m) { return m->len; }

void ny_hmap_free(NyHMap *m) {
    for (int64_t i = 0; i < m->cap; i++) {
        if (m->entries[i].occupied) {
            free(m->entries[i].key);
            free(m->entries[i].value);
        }
    }
    free(m->entries);
    free(m);
}

// Get key at index (for iteration). Returns key ptr, sets *out_len.
const char *ny_hmap_key_at(NyHMap *m, int64_t index, int64_t *out_len) {
    int64_t count = 0;
    for (int64_t i = 0; i < m->cap; i++) {
        if (m->entries[i].occupied) {
            if (count == index) {
                *out_len = m->entries[i].key_len;
                return m->entries[i].key;
            }
            count++;
        }
    }
    *out_len = 0;
    return "";
}
