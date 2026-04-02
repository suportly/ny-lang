// Test: use statement to import another .ny file
use "math_lib.ny";

fn main() -> i32 {
    a := add(10, 20);        // 30
    b := multiply(3, 4);     // 12
    c := square(5);           // 25
    // 30 + 12 - 25 = 17
    // We need something recognizable: 30 + 12 = 42
    return a + b;
}
