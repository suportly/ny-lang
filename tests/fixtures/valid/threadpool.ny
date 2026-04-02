// Test: thread pool — submit work and wait for completion

fn work_item(arg: *u8) -> *u8 {
    // Simple work: sleep briefly
    sleep_ms(1);
    buf := alloc(1);
    return buf;
}

fn main() -> i32 {
    pool := pool_new(4);

    // Submit 8 work items
    i :~ i32 = 0;
    while i < 8 {
        pool_submit(pool, work_item);
        i += 1;
    }

    // Wait for all to complete
    pool_wait(pool);
    pool_free(pool);

    // If we get here without deadlock/crash, pool works
    return 42;
}
