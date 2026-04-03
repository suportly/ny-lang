// Fibonacci benchmark — C equivalent of examples/fibonacci_bench.ny
#include <stdio.h>
#include <time.h>

int fibonacci(int n) {
    if (n <= 1) return n;
    return fibonacci(n - 1) + fibonacci(n - 2);
}

int main() {
    struct timespec start, end;

    printf("=== C Fibonacci Benchmark ===\n\n");

    clock_gettime(CLOCK_MONOTONIC, &start);
    int r35 = fibonacci(35);
    clock_gettime(CLOCK_MONOTONIC, &end);
    long ms35 = (end.tv_sec - start.tv_sec) * 1000 + (end.tv_nsec - start.tv_nsec) / 1000000;
    printf("  fibonacci(35) = %d\n  Time: %ld ms\n\n", r35, ms35);

    clock_gettime(CLOCK_MONOTONIC, &start);
    int r40 = fibonacci(40);
    clock_gettime(CLOCK_MONOTONIC, &end);
    long ms40 = (end.tv_sec - start.tv_sec) * 1000 + (end.tv_nsec - start.tv_nsec) / 1000000;
    printf("  fibonacci(40) = %d\n  Time: %ld ms\n\n", r40, ms40);

    printf("=== Done ===\n");
    return 0;
}
