// Error: alloc expects integer size
fn main() -> i32 {
    buf := alloc(true);
    return 0;
}
