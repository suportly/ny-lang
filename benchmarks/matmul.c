// Matrix multiply benchmark — C equivalent of examples/matmul_bench.ny
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <time.h>

void matmul(int *a, int *b, int *c, int n) {
    for (int i = 0; i < n; i++)
        for (int j = 0; j < n; j++) {
            int sum = 0;
            for (int k = 0; k < n; k++)
                sum += a[i * n + k] * b[k * n + j];
            c[i * n + j] = sum;
        }
}

long checksum(int *m, int n) {
    long total = 0;
    for (int i = 0; i < n * n; i++) total += m[i];
    return total;
}

void bench(int n) {
    int *a = malloc(n * n * sizeof(int));
    int *b = malloc(n * n * sizeof(int));
    int *c = calloc(n * n, sizeof(int));
    for (int i = 0; i < n; i++)
        for (int j = 0; j < n; j++) {
            a[i * n + j] = (i + j) % 7;
            b[i * n + j] = (i + j) % 5;
        }

    struct timespec start, end;
    clock_gettime(CLOCK_MONOTONIC, &start);
    matmul(a, b, c, n);
    clock_gettime(CLOCK_MONOTONIC, &end);
    long ms = (end.tv_sec - start.tv_sec) * 1000 + (end.tv_nsec - start.tv_nsec) / 1000000;

    printf("  %dx%d: %ldms (checksum: %ld)\n", n, n, ms, checksum(c, n));
    free(a); free(b); free(c);
}

int main() {
    printf("=== C Matrix Multiply Benchmark ===\n\n");
    bench(32);
    bench(64);
    bench(128);
    bench(256);
    printf("\n=== Done ===\n");
    return 0;
}
