// Test: implicit numeric widening in arithmetic and comparisons

fn main() -> i32 {
    a : i32 = 10;
    b : i64 = 32;

    // i32 + i64 → auto-widen i32 to i64, result is i64
    sum := a + b;

    // i64 variable initialized with i32 literal
    c : i64 = 0;

    // i64 comparison with i32
    if sum > c {
        return sum as i32;  // 42
    }
    return 0;
}
