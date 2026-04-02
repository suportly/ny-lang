// Test: stdlib math + strings
use "stdlib/math.ny";
use "stdlib/strings.ny";

fn main() -> i32 {
    // Math
    a := abs_i32(-7);       // 7
    b := max_i32(10, 20);   // 20
    c := gcd_i32(48, 18);   // 6
    d := factorial(4);      // 24

    // Strings
    starts := str_starts_with("hello", "hel");
    hello := str_repeat("ab", 2);  // "abab"
    println(hello);

    // 7 + 20 + 6 - 24 = 9
    result :~ i32 = a + b + c - d;

    // starts = true → +1 = 10
    if starts { result = result + 1; }

    // 10 + 32 = 42
    return result + 32;
}
