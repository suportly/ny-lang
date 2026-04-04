// Go-style concurrency: goroutines + typed channels + select
//
// Producers send values concurrently via goroutines.
// Main collects results through channels.
fn producer(ch: *u8, value: i32) {
    sleep_ms(10);
    channel_send(ch, value);
}

fn main() -> i32 {
    ch := channel_new(16);
    // Spawn 3 goroutines — run concurrently on thread pool
    go producer(ch, 10);
    go producer(ch, 14);
    go producer(ch, 18);
    // Receive all 3 results (blocks until each is ready)
    total :~= 0;
    for i in 0..3 {
        total += channel_recv(ch);
    }
    println("total:", total); // 10 + 14 + 18 = 42
    channel_close(ch);
    // Typed channels with method syntax
    tch : chan<i32> = chan_new(4);
    tch.send(42);
    result := tch.recv();
    println("typed channel:", result);
    tch.close();
    return 0;
}
