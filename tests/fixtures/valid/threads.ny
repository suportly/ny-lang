// Test: basic thread creation via pthreads

fn worker() -> *u8 {
    sleep_ms(10);
    buf := alloc(1);
    return buf;
}

fn main() -> i32 {
    t := thread_spawn(worker);
    thread_join(t);
    return 42;
}
