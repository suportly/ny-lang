// Ny Lang runtime: Tensor<f64> — 2D matrix operations

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <math.h>
#include <stdint.h>

typedef struct {
    double *data;
    int64_t rows;
    int64_t cols;
} NyTensor;

NyTensor *ny_tensor_zeros(int64_t rows, int64_t cols) {
    NyTensor *t = (NyTensor *)malloc(sizeof(NyTensor));
    t->rows = rows;
    t->cols = cols;
    t->data = (double *)calloc(rows * cols, sizeof(double));
    return t;
}

NyTensor *ny_tensor_ones(int64_t rows, int64_t cols) {
    NyTensor *t = ny_tensor_zeros(rows, cols);
    for (int64_t i = 0; i < rows * cols; i++) t->data[i] = 1.0;
    return t;
}

NyTensor *ny_tensor_fill(int64_t rows, int64_t cols, double val) {
    NyTensor *t = ny_tensor_zeros(rows, cols);
    for (int64_t i = 0; i < rows * cols; i++) t->data[i] = val;
    return t;
}

NyTensor *ny_tensor_rand(int64_t rows, int64_t cols) {
    NyTensor *t = ny_tensor_zeros(rows, cols);
    for (int64_t i = 0; i < rows * cols; i++)
        t->data[i] = (double)rand() / RAND_MAX;
    return t;
}

NyTensor *ny_tensor_clone(NyTensor *src) {
    NyTensor *t = ny_tensor_zeros(src->rows, src->cols);
    memcpy(t->data, src->data, src->rows * src->cols * sizeof(double));
    return t;
}

// Apply function element-wise (useful for activation functions)
void ny_tensor_apply(NyTensor *t, double (*fn)(double)) {
    int64_t n = t->rows * t->cols;
    for (int64_t i = 0; i < n; i++) t->data[i] = fn(t->data[i]);
}

// Dot product of two vectors (1D tensors stored as 1×N)
double ny_tensor_dot(NyTensor *a, NyTensor *b) {
    int64_t n = a->rows * a->cols;
    double sum = 0.0;
    for (int64_t i = 0; i < n; i++) sum += a->data[i] * b->data[i];
    return sum;
}

// Frobenius norm (sqrt of sum of squares)
double ny_tensor_norm(NyTensor *t) {
    double sum = 0.0;
    int64_t n = t->rows * t->cols;
    for (int64_t i = 0; i < n; i++) sum += t->data[i] * t->data[i];
    return sqrt(sum);
}

void ny_tensor_free(NyTensor *t) {
    if (t) { free(t->data); free(t); }
}

int64_t ny_tensor_rows(NyTensor *t) { return t->rows; }
int64_t ny_tensor_cols(NyTensor *t) { return t->cols; }

double ny_tensor_get(NyTensor *t, int64_t row, int64_t col) {
    return t->data[row * t->cols + col];
}

void ny_tensor_set(NyTensor *t, int64_t row, int64_t col, double val) {
    t->data[row * t->cols + col] = val;
}

// Element-wise add: C = A + B
NyTensor *ny_tensor_add(NyTensor *a, NyTensor *b) {
    NyTensor *c = ny_tensor_zeros(a->rows, a->cols);
    int64_t n = a->rows * a->cols;
    for (int64_t i = 0; i < n; i++) c->data[i] = a->data[i] + b->data[i];
    return c;
}

// Element-wise sub: C = A - B
NyTensor *ny_tensor_sub(NyTensor *a, NyTensor *b) {
    NyTensor *c = ny_tensor_zeros(a->rows, a->cols);
    int64_t n = a->rows * a->cols;
    for (int64_t i = 0; i < n; i++) c->data[i] = a->data[i] - b->data[i];
    return c;
}

// Element-wise mul: C = A * B (Hadamard product)
NyTensor *ny_tensor_mul(NyTensor *a, NyTensor *b) {
    NyTensor *c = ny_tensor_zeros(a->rows, a->cols);
    int64_t n = a->rows * a->cols;
    for (int64_t i = 0; i < n; i++) c->data[i] = a->data[i] * b->data[i];
    return c;
}

// Scalar multiply: C = A * scalar
NyTensor *ny_tensor_scale(NyTensor *a, double scalar) {
    NyTensor *c = ny_tensor_zeros(a->rows, a->cols);
    int64_t n = a->rows * a->cols;
    for (int64_t i = 0; i < n; i++) c->data[i] = a->data[i] * scalar;
    return c;
}

// Matrix multiply: C = A @ B (rows_a x cols_a) @ (rows_b x cols_b) → (rows_a x cols_b)
NyTensor *ny_tensor_matmul(NyTensor *a, NyTensor *b) {
    NyTensor *c = ny_tensor_zeros(a->rows, b->cols);
    for (int64_t i = 0; i < a->rows; i++)
        for (int64_t k = 0; k < a->cols; k++) {
            double a_ik = a->data[i * a->cols + k];
            for (int64_t j = 0; j < b->cols; j++)
                c->data[i * b->cols + j] += a_ik * b->data[k * b->cols + j];
        }
    return c;
}

// Transpose
NyTensor *ny_tensor_transpose(NyTensor *a) {
    NyTensor *c = ny_tensor_zeros(a->cols, a->rows);
    for (int64_t i = 0; i < a->rows; i++)
        for (int64_t j = 0; j < a->cols; j++)
            c->data[j * a->rows + i] = a->data[i * a->cols + j];
    return c;
}

// Sum all elements
double ny_tensor_sum(NyTensor *t) {
    double s = 0.0;
    int64_t n = t->rows * t->cols;
    for (int64_t i = 0; i < n; i++) s += t->data[i];
    return s;
}

// Max element
double ny_tensor_max(NyTensor *t) {
    double m = t->data[0];
    int64_t n = t->rows * t->cols;
    for (int64_t i = 1; i < n; i++) if (t->data[i] > m) m = t->data[i];
    return m;
}

// Min element
double ny_tensor_min(NyTensor *t) {
    double m = t->data[0];
    int64_t n = t->rows * t->cols;
    for (int64_t i = 1; i < n; i++) if (t->data[i] < m) m = t->data[i];
    return m;
}

// Print tensor (for debugging)
void ny_tensor_print(NyTensor *t) {
    printf("Tensor(%ld, %ld)\n", (long)t->rows, (long)t->cols);
    for (int64_t i = 0; i < t->rows; i++) {
        printf("  [");
        for (int64_t j = 0; j < t->cols; j++) {
            if (j > 0) printf(", ");
            printf("%.4f", t->data[i * t->cols + j]);
        }
        printf("]\n");
    }
}
