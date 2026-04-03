// Ny Lang runtime: string helpers + utilities

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
