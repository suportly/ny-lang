// Test: string receiver methods — char_at, contains, starts_with, ends_with

fn main() -> i32 {
    s := "hello world";

    // char_at: 'h' = 104, 'e' = 101
    ch := s.char_at(0);    // 104
    ch2 := s.char_at(1);   // 101

    // contains
    has_world := s.contains("world");    // true
    has_xyz := s.contains("xyz");        // false

    // starts_with
    sw := s.starts_with("hello");    // true
    sw2 := s.starts_with("world");   // false

    // ends_with
    ew := s.ends_with("world");      // true
    ew2 := s.ends_with("hello");     // false

    result :~ i32 = 0;

    // ch - ch2 = 104 - 101 = 3
    result += ch - ch2;

    // booleans: true=1, false=0
    if has_world { result += 10; }
    if !has_xyz { result += 10; }
    if sw { result += 10; }
    if !sw2 { result += 1; }
    if ew { result += 5; }
    if !ew2 { result += 3; }

    // 3 + 10 + 10 + 10 + 1 + 5 + 3 = 42
    return result;
}
