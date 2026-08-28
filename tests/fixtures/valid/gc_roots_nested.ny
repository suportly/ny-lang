// GC roots across call frames: an object held by a caller must survive a
// collection triggered deeper in the call stack, including when the pointer
// only reaches the callee as a parameter.
//
// The churn loop allocates past the 1MB threshold so ny_gc_alloc collects on
// its own, rather than relying on an explicit gc_collect().

struct Box {
    a: i32,
    b: i32,
    c: i32,
    d: i32,
    e: i32,
    f: i32,
    g: i32,
    h: i32,
}

// ~56 bytes per object (32 payload + 24 header): 40000 rounds is well past
// 1MB, so this triggers several automatic collections.
fn churn(rounds: i32) -> i32 {
    i :~ i32 = 0;
    while i < rounds {
        new Box { a: 1, b: 2, c: 3, d: 4, e: 5, f: 6, g: 7, h: 8 };
        i = i + 1;
    }
    return rounds;
}

// `b` arrives as a parameter: it has to be a root for this frame too.
fn use_after_churn(b: *Box) -> i32 {
    churn(40000);
    return b.a;
}

fn main() -> i32 {
    outer := new Box { a: 21, b: 0, c: 0, d: 0, e: 0, f: 0, g: 0, h: 0 };

    // Collections happen one frame below.
    churn(40000);
    if gc_collection_count() < 1 { return 1; }
    if outer.a != 21 { return 2; }

    // Live bytes must still account for `outer` after all that churn.
    if gc_bytes_allocated() <= 0 { return 3; }

    // Collections happen two frames below, pointer passed as an argument.
    if use_after_churn(outer) != 21 { return 4; }

    if outer.a != 21 { return 5; }

    return 42;
}
