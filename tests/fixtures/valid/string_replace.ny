// Test: str.replace(old, new)

fn main() -> i32 {
    s := "hello world hello";

    // Single replacement
    r := s.replace("world", "ny");
    println(r);  // "hello ny hello"
    len1 := r.len();  // 14

    // Multiple occurrences
    r2 := s.replace("hello", "hi");
    println(r2);  // "hi world hi"
    len2 := r2.len();  // 11

    // No match
    r3 := s.replace("xyz", "abc");
    len3 := r3.len();  // 17 (unchanged)

    // len1=14, len2=11, len3=17
    // 14 + 11 + 17 = 42
    return len1 as i32 + len2 as i32 + len3 as i32;
}
