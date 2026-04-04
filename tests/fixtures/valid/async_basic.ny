// Test: basic async/await — async fn returns value via future

async fn compute(x: i32) -> i32 {
    return x * 2;
}

fn main() -> i32 {
    future := compute(21);
    result := await future;
    return result;  // 21 * 2 = 42
}
