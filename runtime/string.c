// Ny Lang runtime: string helpers + utilities

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <time.h>

// Returns monotonic clock in milliseconds (for benchmarking)
long ny_clock_ms(void) {
    struct timespec ts;
    clock_gettime(CLOCK_MONOTONIC, &ts);
    return (long)ts.tv_sec * 1000 + (long)ts.tv_nsec / 1000000;
}

// Split a string by delimiter. Returns array of {ptr, len} pairs.
// out_count receives the number of parts.
// Caller must free the returned array.
typedef struct { const char *ptr; long len; } NyStrSlice;

NyStrSlice *ny_str_split(const char *hay, long hay_len,
                          const char *delim, long delim_len,
                          long *out_count) {
    if (delim_len == 0 || hay_len == 0) {
        NyStrSlice *result = (NyStrSlice *)malloc(sizeof(NyStrSlice));
        result[0].ptr = hay;
        result[0].len = hay_len;
        *out_count = 1;
        return result;
    }

    // Count parts
    long count = 1;
    const char *p = hay;
    const char *end = hay + hay_len;
    while (p + delim_len <= end) {
        if (memcmp(p, delim, delim_len) == 0) {
            count++;
            p += delim_len;
        } else {
            p++;
        }
    }

    NyStrSlice *result = (NyStrSlice *)malloc(count * sizeof(NyStrSlice));
    long idx = 0;
    const char *start = hay;
    p = hay;
    while (p + delim_len <= end) {
        if (memcmp(p, delim, delim_len) == 0) {
            result[idx].ptr = start;
            result[idx].len = p - start;
            idx++;
            p += delim_len;
            start = p;
        } else {
            p++;
        }
    }
    // Last part
    result[idx].ptr = start;
    result[idx].len = end - start;

    *out_count = count;
    return result;
}

// --- Call stack tracking for stack traces ---

#define NY_TRACE_MAX 256
static const char *ny_trace_stack[NY_TRACE_MAX];
static int ny_trace_depth = 0;

void ny_trace_push(const char *name) {
    if (ny_trace_depth < NY_TRACE_MAX) {
        ny_trace_stack[ny_trace_depth++] = name;
    }
}

void ny_trace_pop(void) {
    if (ny_trace_depth > 0) ny_trace_depth--;
}

void ny_trace_print(void) {
    if (ny_trace_depth == 0) return;
    fprintf(stderr, "stack trace:\n");
    for (int i = ny_trace_depth - 1; i >= 0; i--) {
        fprintf(stderr, "  %d: %s()\n", ny_trace_depth - 1 - i, ny_trace_stack[i]);
    }
}

// Remove a file. Returns 0 on success, -1 on failure.
int ny_remove_file(const char *path, long path_len) {
    char *cpath = (char *)malloc(path_len + 1);
    memcpy(cpath, path, path_len);
    cpath[path_len] = '\0';
    int result = remove(cpath);
    free(cpath);
    return result == 0 ? 0 : -1;
}

// Read entire file into a string. Returns malloc'd buffer, sets *out_len.
// Returns NULL on failure.
char *ny_read_file(const char *path, long path_len, long *out_len) {
    // Null-terminate path for fopen
    char *cpath = (char *)malloc(path_len + 1);
    memcpy(cpath, path, path_len);
    cpath[path_len] = '\0';

    FILE *f = fopen(cpath, "rb");
    free(cpath);
    if (!f) { *out_len = 0; return (char *)malloc(1); }

    fseek(f, 0, SEEK_END);
    long size = ftell(f);
    fseek(f, 0, SEEK_SET);

    char *buf = (char *)malloc(size > 0 ? size : 1);
    if (size > 0) fread(buf, 1, size, f);
    fclose(f);

    *out_len = size;
    return buf;
}

// Write string to file. Returns 0 on success, -1 on failure.
int ny_write_file(const char *path, long path_len, const char *data, long data_len) {
    char *cpath = (char *)malloc(path_len + 1);
    memcpy(cpath, path, path_len);
    cpath[path_len] = '\0';

    FILE *f = fopen(cpath, "wb");
    free(cpath);
    if (!f) return -1;

    fwrite(data, 1, data_len, f);
    fclose(f);
    return 0;
}

// Convert f64 to string. Returns malloc'd buffer, sets *out_len.
char *ny_float_to_str(double val, long *out_len) {
    char buf[64];
    int n = snprintf(buf, sizeof(buf), "%.6g", val);
    char *result = (char *)malloc(n);
    memcpy(result, buf, n);
    *out_len = n;
    return result;
}

// Convert string to f64.
double ny_str_to_float(const char *ptr, long len) {
    char buf[128];
    long copy_len = len < 127 ? len : 127;
    memcpy(buf, ptr, copy_len);
    buf[copy_len] = '\0';
    return strtod(buf, NULL);
}

// Replace all occurrences of 'old' in 'haystack' with 'new_s'.
// Returns: pointer to newly allocated string.
// out_len: receives the length of the result string.
char *ny_str_replace(const char *hay, long hay_len,
                     const char *old, long old_len,
                     const char *new_s, long new_len,
                     long *out_len) {
    if (old_len == 0) {
        // Empty pattern: just copy
        char *result = (char *)malloc(hay_len);
        memcpy(result, hay, hay_len);
        *out_len = hay_len;
        return result;
    }

    // Count occurrences
    long count = 0;
    const char *p = hay;
    const char *end = hay + hay_len;
    while (p + old_len <= end) {
        if (memcmp(p, old, old_len) == 0) {
            count++;
            p += old_len;
        } else {
            p++;
        }
    }

    // Calculate result length
    long result_len = hay_len + count * (new_len - old_len);
    char *result = (char *)malloc(result_len > 0 ? result_len : 1);

    // Build result
    const char *src = hay;
    char *dst = result;
    while (src + old_len <= end) {
        if (memcmp(src, old, old_len) == 0) {
            memcpy(dst, new_s, new_len);
            dst += new_len;
            src += old_len;
        } else {
            *dst++ = *src++;
        }
    }
    // Copy remaining bytes
    while (src < end) {
        *dst++ = *src++;
    }

    *out_len = result_len;
    return result;
}
