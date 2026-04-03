// Test: float_to_str / str_to_float

fn main() -> i32 {
    // float to string
    s := float_to_str(3.14);
    println(s);

    // string to float
    f := str_to_float("2.5");

    // s.len() should be 4 ("3.14")
    // f as i32 = 2
    // 4 + 2 + 36 = 42
    return s.len() as i32 + f as i32 + 36;
}
