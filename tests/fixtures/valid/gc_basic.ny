// Phase 26: GC — basic garbage collector test
// Tests: gc_alloc, gc_collect, gc_bytes_allocated, gc_collection_count

fn main() -> i32 {
    // Allocate GC-managed memory (no free needed)
    p1 := gc_alloc(64);
    p2 := gc_alloc(128);
    p3 := gc_alloc(256);

    // Verify bytes were allocated
    bytes := gc_bytes_allocated();
    if bytes <= 0 {
        return 1;
    }

    // Force a collection
    gc_collect();

    // Verify collection happened
    collections := gc_collection_count();
    if collections < 1 {
        return 2;
    }

    // Allocate more — GC should handle cleanup automatically
    i :~ i32 = 0;
    while i < 100 {
        gc_alloc(1024);
        i = i + 1;
    }

    // Another collection
    gc_collect();

    if gc_collection_count() < 2 {
        return 3;
    }

    return 42;
}
