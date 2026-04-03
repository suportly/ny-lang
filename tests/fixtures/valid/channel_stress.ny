// Test: channel stress — producer sends 100 values, consumer sums them

fn producer(ch: *u8) -> *u8 {
    i :~ i32 = 1;
    while i <= 100 {
        channel_send(ch, i);
        i += 1;
    }
    // Signal done
    channel_send(ch, 0);
    buf := alloc(1);
    return buf;
}

fn main() -> i32 {
    ch := channel_new(8);  // Small buffer forces blocking

    t := thread_spawn(producer, ch);

    // Consume until we see 0
    total :~ i32 = 0;
    count :~ i32 = 0;
    loop {
        val := channel_recv(ch);
        if val == 0 { break; }
        total += val;
        count += 1;
    }

    thread_join(t);
    channel_close(ch);

    // sum(1..100) = 5050, count = 100
    // 5050 / 100 - 8 = 50 - 8 = 42
    return total / count - 8;
}
