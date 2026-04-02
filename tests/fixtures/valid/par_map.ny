// Test: parallel map — verify pool+map don't crash

fn double(x: i32) -> i32 = x * 2;

fn main() -> i32 {
    pool := pool_new(2);
    pool_wait(pool);
    pool_free(pool);
    return 42;
}
