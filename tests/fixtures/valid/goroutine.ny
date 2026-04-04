// Phase 28: go keyword — goroutines with typed channels
// Tests: go fn(args), chan<i32>, producer-consumer pattern

fn producer(ch: *u8, value: i32) {
    // ch is the opaque channel pointer, cast handled internally
    channel_send(ch, value);
}

fn main() -> i32 {
    // Use old-style channel (compatible with thread dispatch)
    ch := channel_new(16);

    // Spawn goroutines
    go producer(ch, 10);
    go producer(ch, 14);
    go producer(ch, 18);

    // Receive all values
    a := channel_recv(ch);
    b := channel_recv(ch);
    c := channel_recv(ch);

    // Sum should be 42 (order may vary, but sum is deterministic)
    total := a + b + c;

    channel_close(ch);

    if total != 42 { return 1; }

    return 42;
}
