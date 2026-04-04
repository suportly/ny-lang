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

void ny_map_remove(NyHashMap *m, const char *key, int64_t key_len) {
    uint64_t h = hash_bytes(key, key_len) % m->cap;
    int64_t probes = 0;
    while (m->entries[h].occupied && probes < m->cap) {
        if (m->entries[h].key_len == key_len &&
            memcmp(m->entries[h].key, key, key_len) == 0) {
            free(m->entries[h].key);
            m->entries[h].key = NULL;
            m->entries[h].key_len = 0;
            m->entries[h].value = 0;
            m->entries[h].occupied = 0;
            m->len--;
            return;
        }
        h = (h + 1) % m->cap;
        probes++;
    }
}

// Get the key at position `index` (iterating over occupied entries).
// Returns: pointer to key, sets *out_len. Returns NULL if index out of range.
const char *ny_map_key_at(NyHashMap *m, int64_t index, int64_t *out_len) {
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

// ===== String→String Map (NyStrMap) =====

typedef struct {
    char *key;
    int64_t key_len;
    char *val;
    int64_t val_len;
    int occupied;
} SEntry;

typedef struct {
    SEntry *entries;
    int64_t len;
    int64_t cap;
} NyStrMap;

NyStrMap *ny_smap_new(void) {
    NyStrMap *m = (NyStrMap *)malloc(sizeof(NyStrMap));
    m->entries = (SEntry *)calloc(INIT_CAP, sizeof(SEntry));
    m->len = 0;
    m->cap = INIT_CAP;
    return m;
}

static void smap_grow(NyStrMap *m) {
    int64_t new_cap = m->cap * 2;
    SEntry *new_entries = (SEntry *)calloc(new_cap, sizeof(SEntry));
    for (int64_t i = 0; i < m->cap; i++) {
        if (m->entries[i].occupied) {
            uint64_t h = hash_bytes(m->entries[i].key, m->entries[i].key_len) % new_cap;
            while (new_entries[h].occupied) h = (h + 1) % new_cap;
            new_entries[h] = m->entries[i];
        }
    }
    free(m->entries);
    m->entries = new_entries;
    m->cap = new_cap;
}

void ny_smap_insert(NyStrMap *m, const char *key, int64_t key_len,
                    const char *val, int64_t val_len) {
    if ((double)m->len / m->cap >= LOAD_FACTOR) smap_grow(m);
    uint64_t h = hash_bytes(key, key_len) % m->cap;
    while (m->entries[h].occupied) {
        if (m->entries[h].key_len == key_len && memcmp(m->entries[h].key, key, key_len) == 0) {
            free(m->entries[h].val);
            m->entries[h].val = (char *)malloc(val_len);
            memcpy(m->entries[h].val, val, val_len);
            m->entries[h].val_len = val_len;
            return;
        }
        h = (h + 1) % m->cap;
    }
    m->entries[h].key = (char *)malloc(key_len);
    memcpy(m->entries[h].key, key, key_len);
    m->entries[h].key_len = key_len;
    m->entries[h].val = (char *)malloc(val_len);
    memcpy(m->entries[h].val, val, val_len);
    m->entries[h].val_len = val_len;
    m->entries[h].occupied = 1;
    m->len++;
}

char *ny_smap_get(NyStrMap *m, const char *key, int64_t key_len, int64_t *out_len) {
    uint64_t h = hash_bytes(key, key_len) % m->cap;
    int64_t probes = 0;
    while (m->entries[h].occupied && probes < m->cap) {
        if (m->entries[h].key_len == key_len && memcmp(m->entries[h].key, key, key_len) == 0) {
            *out_len = m->entries[h].val_len;
            return m->entries[h].val;
        }
        h = (h + 1) % m->cap;
        probes++;
    }
    *out_len = 0;
    return "";
}

int ny_smap_contains(NyStrMap *m, const char *key, int64_t key_len) {
    uint64_t h = hash_bytes(key, key_len) % m->cap;
    int64_t probes = 0;
    while (m->entries[h].occupied && probes < m->cap) {
        if (m->entries[h].key_len == key_len && memcmp(m->entries[h].key, key, key_len) == 0) return 1;
        h = (h + 1) % m->cap;
        probes++;
    }
    return 0;
}

int64_t ny_smap_len(NyStrMap *m) { return m->len; }

void ny_smap_free(NyStrMap *m) {
    for (int64_t i = 0; i < m->cap; i++) {
        if (m->entries[i].occupied) {
            free(m->entries[i].key);
            free(m->entries[i].val);
        }
    }
    free(m->entries);
    free(m);
}

void ny_map_free(NyHashMap *m) {
    for (int64_t i = 0; i < m->cap; i++) {
        if (m->entries[i].occupied && m->entries[i].key) {
            free(m->entries[i].key);
        }
    }
    free(m->entries);
    free(m);
}
