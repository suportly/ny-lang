// Test: str_split_count() and str_split_get()

fn main() -> i32 {
    s := "one,two,three";

    count := str_split_count(s, ",");  // 3

    first := str_split_get(s, ",", 0);   // "one"
    second := str_split_get(s, ",", 1);  // "two"
    third := str_split_get(s, ",", 2);   // "three"

    // count=3, first.len=3, second.len=3, third.len=5
    // 3 + 3 + 3 + 5 = 14
    result :~ i32 = count + first.len() as i32 + second.len() as i32 + third.len() as i32;

    // Verify content
    if first.starts_with("one") { result += 10; }
    if second.starts_with("two") { result += 10; }
    if third.starts_with("three") { result += 8; }

    // 14 + 10 + 10 + 8 = 42
    return result;
}
