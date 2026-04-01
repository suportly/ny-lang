// Phase 5: defer + alloc + free

fn main() -> i32 {
    // Test defer with a simple counter
    result :~ i32 = 0;

    // Test that alloc and free work (basic heap allocation)
    buf :~ *u8 = alloc(64);
    defer free(buf);

    // Multiple defers execute in LIFO order
    // We test the core mechanism: defer runs at return
    result = 42;

    return result;
}
