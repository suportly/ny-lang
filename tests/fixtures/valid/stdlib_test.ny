// Test: stdlib math functions
use "stdlib/math.ny";

fn main() -> i32 {
    a := abs_i32(-7);       // 7
    b := max_i32(10, 20);   // 20
    c := min_i32(10, 20);   // 10
    d := gcd_i32(48, 18);   // 6
    e := pow_i32(2, 3);     // 8
    f := factorial(4);      // 24

    // 7 + 20 - 10 + 6 - 8 + 24 + 3 = 42
    return a + b - c + d - e + f + 3;
}
