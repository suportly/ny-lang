// Phase 11: Lambda expressions (non-capturing)

fn apply(f: fn(i32) -> i32, x: i32) -> i32 {
    return f(x);
}

fn main() -> i32 {
    double := |x: i32| -> i32 { return x * 2; };
    result := apply(double, 21);
    return result; // 42
}
