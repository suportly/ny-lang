// Test: channel — producer sends values, main thread receives

fn producer(ch: *u8) -> *u8 {
    channel_send(ch, 10);
    channel_send(ch, 20);
    channel_send(ch, 12);
    buf := alloc(1);
    return buf;
}

fn main() -> i32 {
    ch := channel_new(16);

    t := thread_spawn(producer, ch);

    a := channel_recv(ch);
    b := channel_recv(ch);
    c := channel_recv(ch);

    thread_join(t);
    channel_close(ch);

    return a + b + c;  // 10 + 20 + 12 = 42
}
