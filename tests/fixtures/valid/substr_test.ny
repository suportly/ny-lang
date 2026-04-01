// Test: string substr() method

fn main() -> i32 {
    s : str = "hello world";

    // substr(0, 5) = "hello"
    sub := s.substr(0 as i64, 5 as i64);
    println(sub);

    // substr length should be 5
    sub_len := sub.len();

    // substr(6, 11) = "world"
    sub2 := s.substr(6 as i64, 11 as i64);
    println(sub2);
    sub2_len := sub2.len();

    // 5 + 5 = 10, return 10 + 32 = 42
    return sub_len as i32 + sub2_len as i32 + 32;
}
