// Test: thread pool stress — 32 tasks on 4 worker threads

fn heavy_work(arg: *u8) -> *u8 {
    // Do some actual computation (not just sleep)
    sum :~ i32 = 0;
    i :~ i32 = 0;
    while i < 10000 {
        sum += i;
        i += 1;
    }
    buf := alloc(1);
    return buf;
}

fn main() -> i32 {
    pool := pool_new(4);

    // Submit 32 computation tasks
    i :~ i32 = 0;
    while i < 32 {
        pool_submit(pool, heavy_work);
        i += 1;
    }

    pool_wait(pool);
    pool_free(pool);

    // Survived 32 tasks across 4 threads without crash/deadlock
    return 42;
}
