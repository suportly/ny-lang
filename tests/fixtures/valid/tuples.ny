fn divide(a: i32, b: i32) -> (i32, bool) {
    if b == 0 {
        return (0, false);
    }
    return (a / b, true);
}

fn main() -> i32 {
    (result, ok) := divide(10, 3);
    if ok {
        return result;
    }
    return 0;
}
