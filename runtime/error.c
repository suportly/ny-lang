// Ny Lang runtime: error handling with string messages + stack traces
//
// Usage:
//   code := error_new("division by zero");
//   msg  := error_message(code);
//   trace := error_trace(code);  // stack trace at error creation point

#include <stdlib.h>
#include <stdint.h>
#include <string.h>
#include <stdio.h>

// From string.c — call stack tracking
extern const char *ny_trace_stack[];
extern int ny_trace_depth;

#define MAX_ERRORS 1024

typedef struct {
    const char *message;
    int64_t msg_len;
    char *trace;
    int64_t trace_len;
} NyError;

static NyError g_errors[MAX_ERRORS];
static int32_t g_error_count = 0;

// Create a new error with a message. Captures current stack trace.
int32_t ny_error_new(const char *msg_ptr, int64_t msg_len) {
    if (g_error_count >= MAX_ERRORS) {
        fprintf(stderr, "ny: too many errors (max %d)\n", MAX_ERRORS);
        return -1;
    }
    int32_t code = g_error_count + 1;
    char *copy = (char *)malloc(msg_len + 1);
    if (copy) {
        memcpy(copy, msg_ptr, msg_len);
        copy[msg_len] = '\0';
    }
    g_errors[g_error_count].message = copy;
    g_errors[g_error_count].msg_len = msg_len;

    // Capture stack trace snapshot
    char trace_buf[8192];
    int pos = 0;
    for (int i = ny_trace_depth - 1; i >= 0 && pos < 8000; i--) {
        pos += snprintf(trace_buf + pos, 8192 - pos, "  %d: %s()\n",
                        ny_trace_depth - 1 - i, ny_trace_stack[i]);
    }
    char *trace_copy = NULL;
    if (pos > 0) {
        trace_copy = (char *)malloc(pos + 1);
        if (trace_copy) { memcpy(trace_copy, trace_buf, pos); trace_copy[pos] = '\0'; }
    }
    g_errors[g_error_count].trace = trace_copy;
    g_errors[g_error_count].trace_len = pos;

    g_error_count++;
    return code;
}

// Get the message for an error code.
const char *ny_error_message(int32_t code, int64_t *out_len) {
    if (code <= 0 || code > g_error_count) {
        *out_len = 14;
        return "unknown error";
    }
    *out_len = g_errors[code - 1].msg_len;
    return g_errors[code - 1].message;
}

// Get the stack trace captured at error creation.
const char *ny_error_trace(int32_t code, int64_t *out_len) {
    if (code <= 0 || code > g_error_count || !g_errors[code - 1].trace) {
        *out_len = 0;
        return "";
    }
    *out_len = g_errors[code - 1].trace_len;
    return g_errors[code - 1].trace;
}
