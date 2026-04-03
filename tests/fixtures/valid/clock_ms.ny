// Test: clock_ms() builtin — monotonic timer

fn main() -> i32 {
    start := clock_ms();
    // Do some work
    sum :~ i32 = 0;
    i :~ i32 = 0;
    while i < 1000 {
        sum += i;
        i += 1;
    }
    end := clock_ms();

    // Elapsed should be >= 0 (monotonic)
    elapsed := end - start;
    if elapsed >= 0 {
        return 42;
    }
    return 0;
}
