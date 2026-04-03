// Ny Lang runtime: minimal JSON parser
// Supports: objects, arrays, strings, numbers (int/float), booleans, null

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <ctype.h>

// JSON value types
#define NY_JSON_NULL    0
#define NY_JSON_BOOL    1
#define NY_JSON_INT     2
#define NY_JSON_FLOAT   3
#define NY_JSON_STRING  4
#define NY_JSON_ARRAY   5
#define NY_JSON_OBJECT  6

typedef struct NyJsonValue NyJsonValue;
typedef struct NyJsonPair NyJsonPair;

struct NyJsonPair {
    char *key;
    long key_len;
    NyJsonValue *value;
};

struct NyJsonValue {
    int type;
    union {
        int bool_val;
        long int_val;
        double float_val;
        struct { char *ptr; long len; } str_val;
        struct { NyJsonValue **items; long len; long cap; } arr_val;
        struct { NyJsonPair *pairs; long len; long cap; } obj_val;
    };
};

// Forward declarations
static NyJsonValue *parse_value(const char **p, const char *end);
static void skip_ws(const char **p, const char *end);

static NyJsonValue *make_val(int type) {
    NyJsonValue *v = (NyJsonValue *)calloc(1, sizeof(NyJsonValue));
    v->type = type;
    return v;
}

static void skip_ws(const char **p, const char *end) {
    while (*p < end && isspace((unsigned char)**p)) (*p)++;
}

static NyJsonValue *parse_string(const char **p, const char *end) {
    if (**p != '"') return NULL;
    (*p)++; // skip opening quote
    const char *start = *p;
    while (*p < end && **p != '"') {
        if (**p == '\\') (*p)++; // skip escaped char
        (*p)++;
    }
    long len = *p - start;
    if (*p < end) (*p)++; // skip closing quote
    NyJsonValue *v = make_val(NY_JSON_STRING);
    v->str_val.ptr = (char *)malloc(len);
    memcpy(v->str_val.ptr, start, len);
    v->str_val.len = len;
    return v;
}

static NyJsonValue *parse_number(const char **p, const char *end) {
    const char *start = *p;
    int is_float = 0;
    if (**p == '-') (*p)++;
    while (*p < end && isdigit((unsigned char)**p)) (*p)++;
    if (*p < end && **p == '.') { is_float = 1; (*p)++; while (*p < end && isdigit((unsigned char)**p)) (*p)++; }
    if (*p < end && (**p == 'e' || **p == 'E')) { is_float = 1; (*p)++; if (*p < end && (**p == '+' || **p == '-')) (*p)++; while (*p < end && isdigit((unsigned char)**p)) (*p)++; }

    char buf[64];
    long len = *p - start;
    if (len > 63) len = 63;
    memcpy(buf, start, len);
    buf[len] = '\0';

    if (is_float) {
        NyJsonValue *v = make_val(NY_JSON_FLOAT);
        v->float_val = strtod(buf, NULL);
        return v;
    } else {
        NyJsonValue *v = make_val(NY_JSON_INT);
        v->int_val = strtol(buf, NULL, 10);
        return v;
    }
}

static NyJsonValue *parse_array(const char **p, const char *end) {
    (*p)++; // skip [
    NyJsonValue *v = make_val(NY_JSON_ARRAY);
    v->arr_val.cap = 8;
    v->arr_val.items = (NyJsonValue **)malloc(8 * sizeof(NyJsonValue *));
    v->arr_val.len = 0;

    skip_ws(p, end);
    if (*p < end && **p == ']') { (*p)++; return v; }

    while (*p < end) {
        NyJsonValue *item = parse_value(p, end);
        if (!item) break;
        if (v->arr_val.len >= v->arr_val.cap) {
            v->arr_val.cap *= 2;
            v->arr_val.items = (NyJsonValue **)realloc(v->arr_val.items, v->arr_val.cap * sizeof(NyJsonValue *));
        }
        v->arr_val.items[v->arr_val.len++] = item;
        skip_ws(p, end);
        if (*p < end && **p == ',') { (*p)++; skip_ws(p, end); }
        else break;
    }
    skip_ws(p, end);
    if (*p < end && **p == ']') (*p)++;
    return v;
}

static NyJsonValue *parse_object(const char **p, const char *end) {
    (*p)++; // skip {
    NyJsonValue *v = make_val(NY_JSON_OBJECT);
    v->obj_val.cap = 8;
    v->obj_val.pairs = (NyJsonPair *)malloc(8 * sizeof(NyJsonPair));
    v->obj_val.len = 0;

    skip_ws(p, end);
    if (*p < end && **p == '}') { (*p)++; return v; }

    while (*p < end) {
        skip_ws(p, end);
        NyJsonValue *key = parse_string(p, end);
        if (!key) break;
        skip_ws(p, end);
        if (*p < end && **p == ':') (*p)++;
        skip_ws(p, end);
        NyJsonValue *val = parse_value(p, end);
        if (!val) { free(key); break; }

        if (v->obj_val.len >= v->obj_val.cap) {
            v->obj_val.cap *= 2;
            v->obj_val.pairs = (NyJsonPair *)realloc(v->obj_val.pairs, v->obj_val.cap * sizeof(NyJsonPair));
        }
        NyJsonPair *pair = &v->obj_val.pairs[v->obj_val.len++];
        pair->key = key->str_val.ptr;
        pair->key_len = key->str_val.len;
        pair->value = val;
        free(key); // free the wrapper, not the string

        skip_ws(p, end);
        if (*p < end && **p == ',') { (*p)++; skip_ws(p, end); }
        else break;
    }
    skip_ws(p, end);
    if (*p < end && **p == '}') (*p)++;
    return v;
}

static NyJsonValue *parse_value(const char **p, const char *end) {
    skip_ws(p, end);
    if (*p >= end) return NULL;

    if (**p == '"') return parse_string(p, end);
    if (**p == '[') return parse_array(p, end);
    if (**p == '{') return parse_object(p, end);
    if (**p == '-' || isdigit((unsigned char)**p)) return parse_number(p, end);

    // true/false/null
    if (end - *p >= 4 && memcmp(*p, "true", 4) == 0) {
        *p += 4; NyJsonValue *v = make_val(NY_JSON_BOOL); v->bool_val = 1; return v;
    }
    if (end - *p >= 5 && memcmp(*p, "false", 5) == 0) {
        *p += 5; NyJsonValue *v = make_val(NY_JSON_BOOL); v->bool_val = 0; return v;
    }
    if (end - *p >= 4 && memcmp(*p, "null", 4) == 0) {
        *p += 4; return make_val(NY_JSON_NULL);
    }
    return NULL;
}

// Public API

NyJsonValue *ny_json_parse(const char *data, long data_len) {
    const char *p = data;
    const char *end = data + data_len;
    return parse_value(&p, end);
}

int ny_json_type(NyJsonValue *v) {
    return v ? v->type : NY_JSON_NULL;
}

long ny_json_get_int(NyJsonValue *obj, const char *key, long key_len) {
    if (!obj || obj->type != NY_JSON_OBJECT) return 0;
    for (long i = 0; i < obj->obj_val.len; i++) {
        if (obj->obj_val.pairs[i].key_len == key_len &&
            memcmp(obj->obj_val.pairs[i].key, key, key_len) == 0) {
            NyJsonValue *v = obj->obj_val.pairs[i].value;
            if (v->type == NY_JSON_INT) return v->int_val;
            if (v->type == NY_JSON_FLOAT) return (long)v->float_val;
            return 0;
        }
    }
    return 0;
}

double ny_json_get_float(NyJsonValue *obj, const char *key, long key_len) {
    if (!obj || obj->type != NY_JSON_OBJECT) return 0.0;
    for (long i = 0; i < obj->obj_val.len; i++) {
        if (obj->obj_val.pairs[i].key_len == key_len &&
            memcmp(obj->obj_val.pairs[i].key, key, key_len) == 0) {
            NyJsonValue *v = obj->obj_val.pairs[i].value;
            if (v->type == NY_JSON_FLOAT) return v->float_val;
            if (v->type == NY_JSON_INT) return (double)v->int_val;
            return 0.0;
        }
    }
    return 0.0;
}

// Returns pointer + sets *out_len
char *ny_json_get_str(NyJsonValue *obj, const char *key, long key_len, long *out_len) {
    if (!obj || obj->type != NY_JSON_OBJECT) { *out_len = 0; return ""; }
    for (long i = 0; i < obj->obj_val.len; i++) {
        if (obj->obj_val.pairs[i].key_len == key_len &&
            memcmp(obj->obj_val.pairs[i].key, key, key_len) == 0) {
            NyJsonValue *v = obj->obj_val.pairs[i].value;
            if (v->type == NY_JSON_STRING) {
                *out_len = v->str_val.len;
                return v->str_val.ptr;
            }
            *out_len = 0;
            return "";
        }
    }
    *out_len = 0;
    return "";
}

int ny_json_get_bool(NyJsonValue *obj, const char *key, long key_len) {
    if (!obj || obj->type != NY_JSON_OBJECT) return 0;
    for (long i = 0; i < obj->obj_val.len; i++) {
        if (obj->obj_val.pairs[i].key_len == key_len &&
            memcmp(obj->obj_val.pairs[i].key, key, key_len) == 0) {
            NyJsonValue *v = obj->obj_val.pairs[i].value;
            if (v->type == NY_JSON_BOOL) return v->bool_val;
            return 0;
        }
    }
    return 0;
}

long ny_json_len(NyJsonValue *v) {
    if (!v) return 0;
    if (v->type == NY_JSON_ARRAY) return v->arr_val.len;
    if (v->type == NY_JSON_OBJECT) return v->obj_val.len;
    return 0;
}

NyJsonValue *ny_json_arr_get(NyJsonValue *arr, long index) {
    if (!arr || arr->type != NY_JSON_ARRAY || index < 0 || index >= arr->arr_val.len)
        return NULL;
    return arr->arr_val.items[index];
}

void ny_json_free(NyJsonValue *v) {
    if (!v) return;
    if (v->type == NY_JSON_STRING) { free(v->str_val.ptr); }
    if (v->type == NY_JSON_ARRAY) {
        for (long i = 0; i < v->arr_val.len; i++) ny_json_free(v->arr_val.items[i]);
        free(v->arr_val.items);
    }
    if (v->type == NY_JSON_OBJECT) {
        for (long i = 0; i < v->obj_val.len; i++) {
            free(v->obj_val.pairs[i].key);
            ny_json_free(v->obj_val.pairs[i].value);
        }
        free(v->obj_val.pairs);
    }
    free(v);
}
