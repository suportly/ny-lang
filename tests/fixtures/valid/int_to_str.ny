// Test: int_to_str and string operations

fn main() -> i32 {
    s := int_to_str(42);
    println(s);        // prints "42"
    len := s.len();    // 2

    s2 := int_to_str(100);
    println(s2);       // prints "100"
    len2 := s2.len();  // 3

    // Concatenation with converted strings
    msg := "value=" + s;
    println(msg);      // "value=42"

    return len as i32 + len2 as i32;  // 2 + 3 = 5
}
