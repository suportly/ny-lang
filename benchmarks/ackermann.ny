// Ackermann function benchmark — deep recursion stress test

fn ackermann(m: i32, n: i32) -> i32 {
    if m == 0 { return n + 1; }
    if n == 0 { return ackermann(m - 1, 1); }
    return ackermann(m - 1, ackermann(m, n - 1));
}

fn main() -> i32 {
    start := clock_ms();
    result := ackermann(3, 12);
    elapsed := clock_ms() - start;
    println(f"ackermann(3,12) = {result}");
    println(f"time: {elapsed}ms");
    return 0;
}
