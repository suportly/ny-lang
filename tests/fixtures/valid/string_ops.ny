fn main() -> i32 {
    a : str = "hello";
    b : str = " world";
    c := a + b;
    println(c);
    println(a.len());

    if a == "hello" {
        return 42;
    }
    return 0;
}
