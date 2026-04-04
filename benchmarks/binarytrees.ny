// Binary Trees — allocation/deallocation stress test
// Uses 2 bytes per node as flags (left=1, right=1 means has children)
// plus storing child pointers via pointer arithmetic

// Node layout: 17 bytes = [has_children: u8, left_ptr: 8 bytes, right_ptr: 8 bytes]
// Simplified: just count nodes via recursion depth, alloc/free per node

fn make_tree(depth: i32) -> i32 {
    if depth == 0 { return 1; }
    // Simulate allocation work
    buf := alloc(32);
    *buf = depth as u8;
    left := make_tree(depth - 1);
    right := make_tree(depth - 1);
    free(buf);
    return 1 + left + right;
}

fn main() -> i32 {
    max_depth := 20;
    start := clock_ms();

    // Stretch tree
    stretch := make_tree(max_depth + 1);
    println(f"stretch tree of depth {max_depth + 1}, check: {stretch}");

    depth :~ i32 = 4;
    while depth <= max_depth {
        iterations :~ i32 = 1;
        i :~ i32 = 0;
        while i < max_depth - depth {
            iterations = iterations * 2;
            i += 1;
        }

        check :~ i32 = 0;
        i = 0;
        while i < iterations {
            check += make_tree(depth);
            i += 1;
        }
        println(f"{iterations} trees of depth {depth}, check: {check}");
        depth += 2;
    }

    elapsed := clock_ms() - start;
    println(f"binary-trees (depth {max_depth}): {elapsed}ms");
    return 0;
}
