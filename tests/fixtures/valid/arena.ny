// Test: arena allocator — bump allocation with bulk free

fn main() -> i32 {
    a := arena_new(1024);
    defer arena_free(a);

    // Allocate several buffers — no individual free needed
    buf1 := arena_alloc(a, 64);
    buf2 := arena_alloc(a, 128);
    buf3 := arena_alloc(a, 256);

    // Write to buffers
    *buf1 = 10 as u8;
    *buf2 = 20 as u8;
    *buf3 = 12 as u8;

    // Read back (separate deref and cast to avoid precedence issue)
    v1 := *buf1;
    v2 := *buf2;
    v3 := *buf3;

    // Check bytes used (should be > 0)
    used := arena_bytes_used(a);

    // Reset arena — all allocations recycled
    arena_reset(a);
    used_after := arena_bytes_used(a);  // 0

    // v1 + v2 + v3 = 10 + 20 + 12 = 42
    return v1 as i32 + v2 as i32 + v3 as i32;
}
