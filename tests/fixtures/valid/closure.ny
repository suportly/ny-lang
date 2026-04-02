// Test: closures with variable capture

fn main() -> i32 {
    n := 10;
    multiplier := 3;

    // Lambda captures n and multiplier from outer scope
    compute := |x: i32| -> i32 { return x * multiplier + n; };

    // compute(5) = 5 * 3 + 10 = 25
    a := compute(5);

    // compute(7) = 7 * 3 + 10 = 31
    b := compute(7);

    // 25 - 31 = -6, but we want positive
    // a + b - 14 = 25 + 31 - 14 = 42
    return a + b - 14;
}
