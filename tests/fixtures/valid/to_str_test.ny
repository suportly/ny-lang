// Test: to_str for string building

fn main() -> i32 {
    // Convert int to string and concatenate
    x := 42;
    s := "value=" + to_str(x);
    println(s);

    // Convert float
    f := 3.14;
    s2 := "pi=" + to_str(f);
    println(s2);

    // Convert bool
    b := true;
    s3 := "flag=" + to_str(b);
    println(s3);

    // String concatenation with conversions
    msg := "result: " + to_str(x) + " ok=" + to_str(b);
    println(msg);

    return x;
}
