// Test: math builtins (sqrt, sin, cos, floor, ceil, pow, fabs)

fn main() -> i32 {
    r := sqrt(16.0);     // 4.0
    s := sin(0.0);       // 0.0
    c := cos(0.0);       // 1.0
    f := floor(3.9);     // 3.0
    cl := ceil(3.1);     // 4.0
    p := pow(2.0, 5.0);  // 32.0
    a := fabs(-7.0);     // 7.0

    // 4 + 0 + 1 + 3 + 4 + 32 - 7 = 37, +5 = 42
    return r as i32 + s as i32 + c as i32 + f as i32 + cl as i32 + p as i32 - a as i32 + 5;
}
