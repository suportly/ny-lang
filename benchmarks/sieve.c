#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <time.h>

int sieve(int n) {
    char *flags = (char *)malloc(n + 1);
    memset(flags, 1, n + 1);
    flags[0] = flags[1] = 0;
    for (int i = 2; i * i <= n; i++)
        if (flags[i])
            for (int j = i * i; j <= n; j += i)
                flags[j] = 0;
    int count = 0;
    for (int i = 2; i <= n; i++)
        if (flags[i]) count++;
    free(flags);
    return count;
}

int main() {
    int n = 10000000;
    struct timespec s, e;
    clock_gettime(CLOCK_MONOTONIC, &s);
    int count = sieve(n);
    clock_gettime(CLOCK_MONOTONIC, &e);
    long ms = (e.tv_sec - s.tv_sec) * 1000 + (e.tv_nsec - s.tv_nsec) / 1000000;
    printf("primes up to %d: %d\ntime: %ldms\n", n, count, ms);
    return 0;
}
