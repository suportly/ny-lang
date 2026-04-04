#include <stdio.h>
#include <time.h>

int ackermann(int m, int n) {
    if (m == 0) return n + 1;
    if (n == 0) return ackermann(m - 1, 1);
    return ackermann(m - 1, ackermann(m, n - 1));
}

int main() {
    struct timespec s, e;
    clock_gettime(CLOCK_MONOTONIC, &s);
    int result = ackermann(3, 12);
    clock_gettime(CLOCK_MONOTONIC, &e);
    long ms = (e.tv_sec - s.tv_sec) * 1000 + (e.tv_nsec - s.tv_nsec) / 1000000;
    printf("ackermann(3,12) = %d\ntime: %ldms\n", result, ms);
    return 0;
}
