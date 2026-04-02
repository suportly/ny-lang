// Test: f-string with expression interpolation

fn double(n: i32) -> i32 {
    return n * 2;
}

fn main() -> i32 {
    x := 10;
    y := 32;

    // Arithmetic expression in f-string
    msg := f"sum={x + y}";
    println(msg);

    // Multiplication
    msg2 := f"double={x * 2}";
    println(msg2);

    // Complex expression
    msg3 := f"calc={x + y * 2}";
    println(msg3);

    return x + y;
}
