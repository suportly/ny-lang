// Fibonacci Benchmark — Compare Ny performance with clock_ms()
// Usage: ny run fibonacci_bench.ny
//        ny build fibonacci_bench.ny -O 2 -o fib_bench && ./fib_bench

fn fibonacci(n: i32) -> i32 {
    if n <= 1 { return n; }
    return fibonacci(n - 1) + fibonacci(n - 2);
}

fn main() -> i32 {
    println("=== Ny Fibonacci Benchmark ===");
    println("");

    // Warm up
    warmup := fibonacci(20);

    // Benchmark fib(35)
    n := 35;
    start := clock_ms();
    result := fibonacci(n);
    elapsed := clock_ms() - start;

    println(f"  fibonacci({n}) = {result}");
    println(f"  Time: {elapsed} ms");
    println("");

    // Benchmark fib(40)
    n2 := 40;
    start2 := clock_ms();
    result2 := fibonacci(n2);
    elapsed2 := clock_ms() - start2;

    println(f"  fibonacci({n2}) = {result2}");
    println(f"  Time: {elapsed2} ms");

    println("");
    println("=== Done ===");
    return 0;
}
