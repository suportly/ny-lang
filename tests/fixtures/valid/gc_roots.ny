// GC roots: a collection triggered while locals are live must not free them.
//
// Without shadow-stack roots the mark phase has nothing to trace from, so the
// sweep reclaims every live object and these reads hit freed memory. The loop
// allocates well past the 1MB threshold to force an automatic collection
// rather than relying on an explicit gc_collect() call.

struct Node {
    value: i32,
    tag: i32,
}

fn main() -> i32 {
    // Live across every collection below.
    keep := new Node { value: 42, tag: 7 };
    other := new Node { value: 99, tag: 3 };

    // Allocate ~4MB of garbage: crosses the 1MB threshold several times, so
    // ny_gc_alloc collects automatically while `keep` and `other` are live.
    i :~ i32 = 0;
    while i < 20000 {
        new Node { value: i, tag: 0 };
        i = i + 1;
    }

    // An explicit collection on top of the automatic ones.
    gc_collect();

    // Both must have survived, with their payloads intact.
    if keep.value != 42 { return 1; }
    if keep.tag != 7 { return 2; }
    if other.value != 99 { return 3; }
    if other.tag != 3 { return 4; }

    // A collection must actually have run, otherwise this test proves nothing.
    if gc_collection_count() < 1 { return 5; }

    return 42;
}
