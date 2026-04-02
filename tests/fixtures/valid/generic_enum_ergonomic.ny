// Test: generic enum ergonomics — use base name, not mangled name

enum Option<T> {
    Some(T),
    None,
}

fn find_positive(x: i32) -> Option<i32> {
    if x > 0 {
        return Option::Some(x);   // Not Option_i32::Some(x) !
    }
    return Option::None;
}

fn main() -> i32 {
    result := find_positive(42);

    val := match result {
        Option::Some(v) => v,   // Not Option_i32::Some(v) !
        Option::None => 0,
    };

    return val;
}
