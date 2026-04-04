#include <stdio.h>
#include <stdlib.h>
#include <math.h>
#include <time.h>

double eval_A(int i, int j) { return 1.0 / ((i+j)*(i+j+1)/2 + i + 1); }

void eval_A_times_u(double *u, double *v, int n) {
    for (int i = 0; i < n; i++) {
        double sum = 0;
        for (int j = 0; j < n; j++) sum += eval_A(i,j) * u[j];
        v[i] = sum;
    }
}

void eval_At_times_u(double *u, double *v, int n) {
    for (int i = 0; i < n; i++) {
        double sum = 0;
        for (int j = 0; j < n; j++) sum += eval_A(j,i) * u[j];
        v[i] = sum;
    }
}

void eval_AtA_times_u(double *u, double *v, double *tmp, int n) {
    eval_A_times_u(u, tmp, n);
    eval_At_times_u(tmp, v, n);
}

int main() {
    int n = 2000;
    double *u = malloc(n * sizeof(double));
    double *v = malloc(n * sizeof(double));
    double *tmp = malloc(n * sizeof(double));
    for (int i = 0; i < n; i++) { u[i] = 1.0; v[i] = 0; tmp[i] = 0; }

    struct timespec s, e;
    clock_gettime(CLOCK_MONOTONIC, &s);
    for (int i = 0; i < 10; i++) { eval_AtA_times_u(u,v,tmp,n); eval_AtA_times_u(v,u,tmp,n); }
    double vBv = 0, vv = 0;
    for (int i = 0; i < n; i++) { vBv += u[i]*v[i]; vv += v[i]*v[i]; }
    clock_gettime(CLOCK_MONOTONIC, &e);
    long ms = (e.tv_sec-s.tv_sec)*1000 + (e.tv_nsec-s.tv_nsec)/1000000;
    printf("%.9f\n", sqrt(vBv/vv));
    printf("spectral-norm (n=%d): %ldms\n", n, ms);
    free(u); free(v); free(tmp);
    return 0;
}
