// Parallel Sum — goroutines + channels
// Splits computation across goroutines and collects via channel.
//
// Usage: ny run parallel_sum.ny
fn partial_sum(ch: *u8, start: i32, end: i32) {
    total :~= 0;
    for i in start..end {
        total += i;
    }
    channel_send(ch, total);
}

fn main() -> i32 {
    num_workers := 4;
    range_size := 10000;
    ch := channel_new(16);
    println("=== Parallel Sum Demo ===");
    println("workers:", num_workers);
    // Sequential baseline
    seq_start := clock_ms();
    seq_total :~= 0;
    for i in 0..range_size * num_workers {
        seq_total += i;
    }
    seq_time := clock_ms() - seq_start;
    println("sequential:", seq_total, "(" + to_str(seq_time) + "ms)");
    // Parallel with goroutines
    par_start := clock_ms();
    for w in 0..num_workers {
        start := w * range_size;
        end := start + range_size;
        go partial_sum(ch, start, end);
    }
    // Collect results
    par_total :~= 0;
    for i in 0..num_workers {
        par_total += channel_recv(ch);
    }
    par_time := clock_ms() - par_start;
    channel_close(ch);
    println("parallel: ", par_total, "(" + to_str(par_time) + "ms)");
    if seq_total == par_total {
        println("Results match!");
    } else {
        println("ERROR: results differ!");
    }
    return 0;
}
