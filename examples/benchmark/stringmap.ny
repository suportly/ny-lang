// stringmap.ny — A simple string-to-int map using parallel Vec arrays
// This demonstrates: Vec, str comparison, for loops, structs, impl

struct StringMap {
    len: i32,
    cap: i32,
}

// Since we can't have Vec<str> fields in structs yet,
// we use parallel global-style Vecs passed as params

fn map_new() -> StringMap {
    return StringMap { len: 0, cap: 64 };
}

// For a real map we'd need Vec<str> — let's use a simpler approach:
// fixed-size array-based map with linear search

fn map_find_index(keys: [64]str, len: i32, key: str) -> i32 {
    i :~ i32 = 0;
    while i < len {
        if keys[i] == key {
            return i;
        }
        i += 1;
    }
    return 0 - 1; // -1 = not found
}
