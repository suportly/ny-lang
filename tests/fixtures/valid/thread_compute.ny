// Test: parallel computation with threads
// Two threads each compute partial sums, main thread combines results via channel

fn compute_sum(ch: *u8) -> *u8 {
    // Compute sum 1..50 = 1275
    total :~ i32 = 0;
    i :~ i32 = 1;
    while i <= 50 {
        total += i;
        i += 1;
    }
    channel_send(ch, total);
    buf := alloc(1);
    return buf;
}

fn main() -> i32 {
    ch := channel_new(4);

    // Launch 2 threads, each computing sum(1..50) = 1275
    t1 := thread_spawn(compute_sum, ch);
    t2 := thread_spawn(compute_sum, ch);

    // Receive both results
    r1 := channel_recv(ch);
    r2 := channel_recv(ch);

    thread_join(t1);
    thread_join(t2);
    channel_close(ch);

    // Both should be 1275
    // r1 + r2 = 2550
    // 2550 / 50 - 9 = 51 - 9 = 42
    return (r1 + r2) / 50 - 9;
}
