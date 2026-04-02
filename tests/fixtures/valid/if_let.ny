// Test: if let pattern matching

enum Option {
    Some(i32),
    None,
}

fn find_positive(x: i32) -> Option {
    if x > 0 {
        return Option::Some(x);
    }
    return Option::None;
}

fn main() -> i32 {
    result := find_positive(42);

    if let Option::Some(val) = result {
        return val;  // 42
    } else {
        return 0;
    }
}
