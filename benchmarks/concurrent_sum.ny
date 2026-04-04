// Benchmark: concurrent sum via goroutines + channels
// Compare with concurrent_sum.go for Go comparison
//
// N workers each sum a range, results collected via channel.
// ny build benchmarks/concurrent_sum.ny -O 2 -o concurrent_sum && ./concurrent_sum

fn worker(ch: *u8, start: i64, end: i64) {
    total :~ i64 = 0;
    i :~ i64 = start;
    while i < end {
        total = total + i;
        i = i + 1;
    }
    channel_send(ch, total as i32);
}

fn main() -> i32 {
    n : i64 = 100000000;
    num_workers : i64 = 8;
    chunk := n / num_workers;

    ch := channel_new(16);

    start := clock_ms();

    i :~ i64 = 0;
    while i < num_workers {
        lo := i * chunk;
        hi := lo + chunk;
        go worker(ch, lo, hi);
        i = i + 1;
    }

    total :~ i64 = 0;
    i = 0;
    while i < num_workers {
        total = total + channel_recv(ch) as i64;
        i = i + 1;
    }

    elapsed := clock_ms() - start;
    channel_close(ch);

    println(f"sum(0..{n}) = {total}");
    println(f"workers: {num_workers}");
    println(f"time: {elapsed}ms");
    return 0;
}
