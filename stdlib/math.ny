// stdlib/math.ny — Math utility functions

fn abs_i32(x: i32) -> i32 {
    if x < 0 { return 0 - x; }
    return x;
}

fn min_i32(a: i32, b: i32) -> i32 {
    if a < b { return a; }
    return b;
}

fn max_i32(a: i32, b: i32) -> i32 {
    if a > b { return a; }
    return b;
}

fn clamp_i32(val: i32, lo: i32, hi: i32) -> i32 {
    if val < lo { return lo; }
    if val > hi { return hi; }
    return val;
}

fn gcd_i32(a: i32, b: i32) -> i32 {
    x :~ i32 = abs_i32(a);
    y :~ i32 = abs_i32(b);
    while y != 0 {
        temp := y;
        y = x % y;
        x = temp;
    }
    return x;
}

fn pow_i32(base: i32, exp: i32) -> i32 {
    if exp == 0 { return 1; }
    result :~ i32 = 1;
    i :~ i32 = 0;
    while i < exp {
        result = result * base;
        i += 1;
    }
    return result;
}

fn factorial(n: i32) -> i32 {
    if n <= 1 { return 1; }
    return n * factorial(n - 1);
}
