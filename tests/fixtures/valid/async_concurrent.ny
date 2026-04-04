// Test: concurrent async — spawn two futures, await both

async fn double(x: i32) -> i32 {
    return x * 2;
}

async fn triple(x: i32) -> i32 {
    return x * 3;
}

fn main() -> i32 {
    // Spawn two async tasks concurrently
    f1 := double(7);   // will return 14
    f2 := triple(7);   // will return 21

    // Both run in the thread pool in parallel
    a := await f1;  // 14
    b := await f2;  // 21

    // But we need to add 7 to get 42
    return a + b + 7;  // 14 + 21 + 7 = 42
}
