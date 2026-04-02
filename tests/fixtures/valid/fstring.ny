// Test: f-string interpolation

fn main() -> i32 {
    name := "Ny";
    version := 42;

    // f-string: text {expr} text
    msg := f"Hello {name}, version {version}!";
    println(msg);

    // Simple interpolation
    x := 10;
    y := 32;
    result := f"x={x} y={y}";
    println(result);

    return version;
}
