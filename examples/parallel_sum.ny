// Parallel Sum — Demonstrates: threads, channels, clock_ms
// Splits a computation across multiple threads and combines results.
//
// Usage: ny run parallel_sum.ny

fn partial_sum(ch: *u8) -> *u8 {
    // Each thread computes sum of 1..10000
    total :~ i32 = 0;
    i :~ i32 = 1;
    while i <= 10000 {
        total += i;
        i += 1;
    }
    channel_send(ch, total);
    buf := alloc(1);
    return buf;
}

fn main() -> i32 {
    num_threads := 4;
    ch := channel_new(16);

    println("=== Parallel Sum Demo ===");
    println(f"  Threads: {num_threads}");
    println("");

    // Sequential baseline
    println("[1] Sequential...");
    seq_start := clock_ms();
    seq_total :~ i32 = 0;
    t :~ i32 = 0;
    while t < num_threads {
        i :~ i32 = 1;
        while i <= 10000 {
            seq_total += i;
            i += 1;
        }
        t += 1;
    }
    seq_time := clock_ms() - seq_start;
    println(f"  Result: {seq_total}");
    println(f"  Time: {seq_time}ms");

    // Parallel
    println("");
    println("[2] Parallel (4 threads)...");
    par_start := clock_ms();

    handles :~ Vec<i64> = vec_new();
    i :~ i32 = 0;
    while i < num_threads {
        h := thread_spawn(partial_sum, ch);
        handles.push(h as i64);
        i += 1;
    }

    // Collect results
    par_total :~ i32 = 0;
    i = 0;
    while i < num_threads {
        par_total += channel_recv(ch);
        i += 1;
    }

    // Join threads
    i = 0;
    while i < num_threads {
        thread_join(handles.get(i));
        i += 1;
    }

    par_time := clock_ms() - par_start;
    channel_close(ch);

    println(f"  Result: {par_total}");
    println(f"  Time: {par_time}ms");

    // Verify
    println("");
    if seq_total == par_total {
        println("  Results match!");
    } else {
        println("  ERROR: results differ!");
    }

    println("");
    println("=== Done ===");
    return 0;
}
