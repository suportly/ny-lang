// Test: empty expression in f-string should error

fn main() -> i32 {
    x := 10;
    msg := f"empty={}";
    println(msg);
    return 0;
}
