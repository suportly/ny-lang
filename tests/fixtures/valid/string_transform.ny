// Test: str.trim(), str.to_upper(), str.to_lower()

fn main() -> i32 {
    // trim
    s := "  hello  ";
    trimmed := s.trim();
    trim_len := trimmed.len();  // 5 ("hello")

    // to_upper
    lower := "hello";
    upper := lower.to_upper();
    // 'H' = 72
    ch := upper.char_at(0);

    // to_lower
    mixed := "WORLD";
    low := mixed.to_lower();
    // 'w' = 119
    ch2 := low.char_at(0);

    // trim_len=5, ch=72, ch2=119
    // 5 + 72 - 119 = -42, abs = 42
    // Actually: ch2 - ch - trim_len = 119 - 72 - 5 = 42
    return ch2 - ch - trim_len as i32;
}
