// Phase 11: Unsafe pointer operations with alloc/free

fn main() -> i32 {
    // Allocate a buffer and write to it manually
    buf := alloc(32);
    defer free(buf);

    // Store a value through the pointer
    *buf = 42 as u8;

    // Read it back
    val := *buf;

    return val as i32;
}
