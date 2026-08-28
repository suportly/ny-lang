// Safepoints: a collection must be able to stop threads that are running Ny
// code, not just those that happen to be allocating.
//
// The spinner goroutine holds a rooted object and then loops without
// allocating anything. Before safepoints it would never reach a point where
// the collector could stop it, so a stop-the-world collection had no way to
// know its roots were stable. It also must not deadlock: the collector waits
// for every participating thread to park, and a thread that never polls
// would hang the program.

struct Cell {
    a: i32,
    b: i32,
    c: i32,
    d: i32,
    e: i32,
    f: i32,
    g: i32,
    h: i32,
}

// Roots an object, then spins in a pure compute loop — no allocation, so the
// only way this thread parks is a safepoint poll on the loop back-edge.
fn spinner(ch: *u8, seed: i32) {
    mine := new Cell { a: seed, b: 0, c: 0, d: 0, e: 0, f: 0, g: 0, h: 0 };

    acc :~ i32 = 0;
    i :~ i32 = 0;
    while i < 3000000 {
        acc = acc + i % 7;
        i = i + 1;
    }

    // Defeat any chance of the loop being optimised away entirely.
    if acc < 0 { channel_send(ch, 0); return; }

    channel_send(ch, mine.a);
}

// Allocates hard, forcing collections while the spinners are mid-loop.
fn allocator(ch: *u8, seed: i32) {
    mine := new Cell { a: seed, b: 0, c: 0, d: 0, e: 0, f: 0, g: 0, h: 0 };

    i :~ i32 = 0;
    while i < 30000 {
        new Cell { a: i, b: 1, c: 2, d: 3, e: 4, f: 5, g: 6, h: 7 };
        i = i + 1;
    }

    channel_send(ch, mine.a);
}

fn main() -> i32 {
    ch := channel_new(16);

    anchor := new Cell { a: 6, b: 0, c: 0, d: 0, e: 0, f: 0, g: 0, h: 0 };

    go spinner(ch, 10);
    go spinner(ch, 11);
    go allocator(ch, 15);

    a := channel_recv(ch);
    b := channel_recv(ch);
    c := channel_recv(ch);

    channel_close(ch);

    // 10 + 11 + 15 = 36; each goroutine's rooted object must have survived.
    if a + b + c != 36 { return 1; }

    // The main thread's root survived collections driven by other threads.
    if anchor.a != 6 { return 2; }

    if gc_collection_count() < 1 { return 3; }

    return 42;
}
