// Error: free expects a pointer
fn main() -> i32 {
    x : i32 = 42;
    free(x);
    return 0;
}
