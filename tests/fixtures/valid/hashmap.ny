// Test: HashMap (string -> int) via runtime library

fn main() -> i32 {
    m := map_new();

    map_insert(m, "one", 1);
    map_insert(m, "two", 2);
    map_insert(m, "three", 3);

    a := map_get(m, "one");     // 1
    b := map_get(m, "two");     // 2
    c := map_get(m, "three");   // 3

    has := map_contains(m, "two");   // true
    missing := map_contains(m, "four"); // false

    len := map_len(m);  // 3

    // a + b + c = 6, len = 3
    // 6 + 3 = 9, +1 (has) +1 (!missing) = 11
    result :~ i32 = a + b + c + len as i32;
    if has {
        result += 1;
    }
    if !missing {
        result += 1;
    }
    return result;  // 11
}
