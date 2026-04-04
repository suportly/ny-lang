// Test: Vec<str>.join(separator) -> str

fn main() -> i32 {
    v :~ Vec<str> = vec_new();
    v.push("a");
    v.push("b");
    v.push("c");

    joined := v.join("-");
    // "a-b-c" = 5 chars

    csv := v.join(", ");
    // "a, b, c" = 7 chars

    // 5 + 7 + 30 = 42
    return joined.len() as i32 + csv.len() as i32 + 30;
}
