// Phase 28: Typed channels — chan<T> with .send(), .recv(), .close()
// Tests: chan<i32> with method syntax, blocking send/recv

fn main() -> i32 {
    // Create a typed channel
    ch : chan<i32> = chan_new(4);

    // Send values using method syntax
    ch.send(10);
    ch.send(20);
    ch.send(12);

    // Receive values
    a := ch.recv();
    b := ch.recv();
    c := ch.recv();

    if a != 10 { return 1; }
    if b != 20 { return 2; }
    if c != 12 { return 3; }

    // Verify sum
    if a + b + c != 42 { return 4; }

    // Close the channel
    ch.close();

    return 42;
}
