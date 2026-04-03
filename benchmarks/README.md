# Ny Lang Benchmarks

Benchmarks comparing Ny with C (gcc) on x86-64 Linux.

**System**: Linux 6.8, AMD/Intel x86-64, Ny compiled with LLVM 18, C compiled with GCC.

## How to Run

```bash
# Ny benchmarks
ny build examples/fibonacci_bench.ny -O 2 -o fib_bench && ./fib_bench
ny build examples/matmul_bench.ny -O 2 -o matmul_bench && ./matmul_bench

# C equivalents
gcc -O2 -o fib_c benchmarks/fib.c && ./fib_c
gcc -O2 -o matmul_c benchmarks/matmul.c && ./matmul_c
```

## Results

### Fibonacci (recursive, no memoization)

| | Ny -O0 | Ny -O1 | **Ny -O2** | Ny -O3 | **C -O2** | C -O3 |
|---|--------|--------|------------|--------|-----------|-------|
| fib(35) | 88ms | 23ms | **33ms** | 33ms | **20ms** | 28ms |
| fib(40) | 975ms | 274ms | **432ms** | 387ms | **249ms** | 252ms |

**Ny -O2 vs C -O2**: ~1.7x on fib(40)
**Ny -O1 vs C -O2**: ~1.1x on fib(40) — nearly identical!

Note: Ny -O1 is faster than Ny -O2 on this benchmark because -O2 enables
additional passes that don't help pure recursion.

### Matrix Multiply (naive triple-loop, i32)

| Size | **Ny -O2** | **C -O2** | Ratio |
|------|-----------|-----------|-------|
| 32x32 | <1ms | <1ms | ~1.0x |
| 64x64 | <1ms | <1ms | ~1.0x |
| 128x128 | 2ms | 2ms | ~1.0x |
| **256x256** | **22ms** | **18ms** | **1.2x** |

**Ny -O2 vs C -O2**: ~1.2x on 256x256 matmul — nearly identical performance.

Checksums verified identical between Ny and C.

### Summary

| Benchmark | Ny -O2 vs C -O2 |
|-----------|-----------------|
| Recursive fibonacci | 1.7x |
| Matrix multiply | 1.2x |

Ny compiles through the same LLVM backend as Clang. On compute-heavy code,
performance is within 1.2x-1.7x of C, depending on the workload pattern.
The gap is primarily due to Ny's bounds checking on Vec access (which C doesn't do).
