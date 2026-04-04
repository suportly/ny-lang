// Test: HashMap iteration via map_key_at + map_remove + map_free

fn main() -> i32 {
    m := map_new();
    defer map_free(m);

    map_insert(m, "a", 10);
    map_insert(m, "b", 20);
    map_insert(m, "c", 12);

    // Iterate and sum values
    total :~ i32 = 0;
    len := map_len(m) as i32;
    i :~ i32 = 0;
    while i < len {
        key := map_key_at(m, i);
        total += map_get(m, key);
        i += 1;
    }

    // Remove a key
    map_remove(m, "b");
    after_len := map_len(m);

    // total=42, after_len=2
    // 42 + 2 - 2 = 42
    return total + after_len as i32 - 2;
}
