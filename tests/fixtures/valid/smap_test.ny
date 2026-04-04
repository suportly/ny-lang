// Test: String→String Map (smap)

fn main() -> i32 {
    m := smap_new();
    defer smap_free(m);

    smap_insert(m, "greeting", "hello");
    smap_insert(m, "target", "world");

    greeting := smap_get(m, "greeting");
    target := smap_get(m, "target");

    // greeting.len()=5, target.len()=5, smap_len=2
    // 5 + 5 + 2 + 30 = 42
    return greeting.len() as i32 + target.len() as i32 + smap_len(m) as i32 + 30;
}
