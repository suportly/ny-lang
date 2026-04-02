// Test: while let pattern matching in loops

enum Option {
    Some(i32),
    None,
}

fn countdown(n: i32) -> Option {
    if n > 0 {
        return Option::Some(n);
    }
    return Option::None;
}

fn main() -> i32 {
    total :~ i32 = 0;
    i :~ i32 = 5;

    while let Option::Some(val) = countdown(i) {
        total += val;
        i -= 1;
    }

    // total = 5 + 4 + 3 + 2 + 1 = 15
    // 15 + 27 = 42
    return total + 27;
}
