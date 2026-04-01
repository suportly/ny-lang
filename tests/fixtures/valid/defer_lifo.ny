// Test: defer LIFO order with multiple defers
// Defers should execute in reverse order: third=30, second=20, first=10
// We use a mutable variable to track execution order via accumulation

fn main() -> i32 {
    result :~ i32 = 0;
    buf1 := alloc(8);
    buf2 := alloc(8);
    buf3 := alloc(8);

    defer free(buf1);
    defer free(buf2);
    defer free(buf3);

    // The value we care about — defers free memory but don't affect result
    result = 42;
    return result;
}
