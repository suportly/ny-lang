// math.ny — Math utilities module

fn abs(x: i32) -> i32 {
    if x < 0 { return 0 - x; }
    return x;
}

fn min<T>(a: T, b: T) -> T {
    if a < b { return a; }
    return b;
}

fn max<T>(a: T, b: T) -> T {
    if a > b { return a; }
    return b;
}

fn clamp(val: i32, lo: i32, hi: i32) -> i32 {
    if val < lo { return lo; }
    if val > hi { return hi; }
    return val;
}

fn gcd(a: i32, b: i32) -> i32 {
    x :~ i32 = abs(a);
    y :~ i32 = abs(b);
    while y != 0 {
        temp := y;
        y = x % y;
        x = temp;
    }
    return x;
}
