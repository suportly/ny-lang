// Phase 32: for key, value in map — Go-style map iteration

fn main() -> i32 {
    m := map_new();
    map_insert(m, "a", 10);
    map_insert(m, "b", 14);
    map_insert(m, "c", 18);

    total :~ i32 = 0;
    for key, value in m {
        total = total + value;
    }

    map_free(m);

    // 10 + 14 + 18 = 42
    if total != 42 { return 1; }
    return 42;
}
