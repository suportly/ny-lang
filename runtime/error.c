// Ny Lang runtime: error handling with string messages
//
// Errors are integer codes (compatible with enum Err(i32)) that map to
// string messages via a thread-local table. This enables rich error messages
// without changing the enum layout.
//
// Usage:
//   code := error_new("division by zero");
//   msg  := error_message(code);

#include <stdlib.h>
#include <stdint.h>
#include <string.h>
#include <stdio.h>

#define MAX_ERRORS 1024

typedef struct {
    const char *message;
    int64_t msg_len;
} NyError;

static NyError g_errors[MAX_ERRORS];
static int32_t g_error_count = 0;

// Create a new error with a message. Returns an error code (1-based).
int32_t ny_error_new(const char *msg_ptr, int64_t msg_len) {
    if (g_error_count >= MAX_ERRORS) {
        fprintf(stderr, "ny: too many errors (max %d)\n", MAX_ERRORS);
        return -1;
    }
    int32_t code = g_error_count + 1; // 1-based (0 = no error)
    char *copy = (char *)malloc(msg_len + 1);
    if (copy) {
        memcpy(copy, msg_ptr, msg_len);
        copy[msg_len] = '\0';
    }
    g_errors[g_error_count].message = copy;
    g_errors[g_error_count].msg_len = msg_len;
    g_error_count++;
    return code;
}

// Get the message for an error code.
// Returns pointer to the message string. Sets *out_len to message length.
const char *ny_error_message(int32_t code, int64_t *out_len) {
    if (code <= 0 || code > g_error_count) {
        *out_len = 14;
        return "unknown error";
    }
    *out_len = g_errors[code - 1].msg_len;
    return g_errors[code - 1].message;
}
