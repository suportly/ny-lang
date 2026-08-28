// GC roots: a collection must not free objects a live local still points at.
//
// Without shadow-stack roots the mark phase has nothing to trace from, so the
// sweep reclaims the whole heap. The check below is deterministic rather than
// relying on reading freed memory: it compares the live byte count across a
// collection. With roots, the two survivors keep their bytes allocated; with
// no roots, everything is swept and the count drops to zero.

struct Node {
    value: i32,
    tag: i32,
}

fn main() -> i32 {
    // Live across the collection below.
    keep := new Node { value: 42, tag: 7 };
    other := new Node { value: 99, tag: 3 };

    before := gc_bytes_allocated();
    if before <= 0 { return 1; }

    gc_collect();

    // The two survivors must still be accounted for. A rootless collector
    // sweeps them and reports 0 bytes live.
    after := gc_bytes_allocated();
    if after <= 0 { return 2; }

    // A collection must actually have run, otherwise this proves nothing.
    if gc_collection_count() < 1 { return 3; }

    // Values must survive intact.
    if keep.value != 42 { return 4; }
    if keep.tag != 7 { return 5; }
    if other.value != 99 { return 6; }
    if other.tag != 3 { return 7; }

    return 42;
}
