// GC under goroutines: several threads allocating at once must not corrupt
// the collector's structures.
//
// Before per-thread shadow stacks and the heap mutex this was doubly broken:
// concurrent ny_gc_alloc calls spliced into the same intrusive object list
// without synchronisation, and the single global shadow stack meant one
// thread's root_pop dropped another thread's entries. Both corrupt memory
// rather than merely losing objects.
//
// Each worker allocates past the 1MB threshold, so collections fire from
// several threads while the others are still allocating and holding roots.

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

// Allocates its own garbage while keeping one object rooted throughout, then
// reports whether that object survived intact.
fn worker(ch: *u8, seed: i32) {
    mine := new Cell { a: seed, b: 0, c: 0, d: 0, e: 0, f: 0, g: 0, h: 0 };

    i :~ i32 = 0;
    while i < 20000 {
        new Cell { a: i, b: 1, c: 2, d: 3, e: 4, f: 5, g: 6, h: 7 };
        i = i + 1;
    }

    // `mine` was live across every collection triggered above, on this thread
    // and on the others.
    channel_send(ch, mine.a);
}

fn main() -> i32 {
    ch := channel_new(16);

    // Keep a rooted object on the main thread too, live for the whole run.
    anchor := new Cell { a: 7, b: 0, c: 0, d: 0, e: 0, f: 0, g: 0, h: 0 };

    go worker(ch, 10);
    go worker(ch, 14);
    go worker(ch, 11);

    a := channel_recv(ch);
    b := channel_recv(ch);
    c := channel_recv(ch);

    channel_close(ch);

    // Each worker's own object must have survived: 10 + 14 + 11 = 35.
    if a + b + c != 35 { return 1; }

    // The main thread's root must have survived collections run by workers.
    if anchor.a != 7 { return 2; }

    // Collections must actually have happened.
    if gc_collection_count() < 1 { return 3; }

    return 42;
}
