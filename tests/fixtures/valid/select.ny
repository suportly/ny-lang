// Phase 31: select statement — channel multiplexing
// Tests: select over multiple typed channels

fn main() -> i32 {
    ch1 : chan<i32> = chan_new(4);
    ch2 : chan<i32> = chan_new(4);

    // Pre-load both channels
    ch1.send(20);
    ch2.send(22);

    // First select: gets from whichever is ready first (both are ready)
    total :~ i32 = 0;
    select {
        v := ch1.recv() => {
            total = total + v;
        },
        v := ch2.recv() => {
            total = total + v;
        },
    }

    // Second select: gets from the remaining channel
    select {
        v := ch1.recv() => {
            total = total + v;
        },
        v := ch2.recv() => {
            total = total + v;
        },
    }

    ch1.close();
    ch2.close();

    // 20 + 22 = 42
    if total != 42 { return 1; }
    return 42;
}
