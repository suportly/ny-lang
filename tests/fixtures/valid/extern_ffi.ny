// Test: extern FFI — call libc functions directly

extern {
    fn abs(x: i32) -> i32;
    fn rand() -> i32;
    fn srand(seed: i32);
}

fn main() -> i32 {
    // abs(-42) should return 42
    result := abs(-42);
    return result;
}
