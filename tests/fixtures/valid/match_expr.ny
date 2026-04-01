fn describe(x: i32) -> i32 {
    return match x {
        0 => 10,
        1 => 20,
        2 => 30,
        _ => 99,
    };
}

fn main() -> i32 {
    a := describe(1);
    b := describe(5);
    return a + b;
}
