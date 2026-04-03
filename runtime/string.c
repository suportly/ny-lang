// Ny Lang runtime: string helper functions

#include <stdlib.h>
#include <string.h>

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
