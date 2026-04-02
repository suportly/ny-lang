// stdlib/strings.ny — String utility functions

fn str_repeat(s: str, n: i32) -> str {
    result :~ str = "";
    i :~ i32 = 0;
    while i < n {
        result = result + s;
        i += 1;
    }
    return result;
}

fn str_starts_with(s: str, prefix: str) -> bool {
    if prefix.len() > s.len() {
        return false;
    }
    sub := s.substr(0 as i64, prefix.len());
    return sub == prefix;
}

fn str_ends_with(s: str, suffix: str) -> bool {
    if suffix.len() > s.len() {
        return false;
    }
    start := s.len() - suffix.len();
    sub := s.substr(start, s.len());
    return sub == suffix;
}

fn str_contains(s: str, needle: str) -> bool {
    slen := s.len();
    nlen := needle.len();
    if nlen > slen { return false; }
    if nlen == 0 as i64 { return true; }

    i :~ i64 = 0;
    while i + nlen <= slen {
        sub := s.substr(i, i + nlen);
        if sub == needle {
            return true;
        }
        i += 1 as i64;
    }
    return false;
}

fn str_pad_left(s: str, total_len: i64, pad_char: str) -> str {
    current := s.len();
    if current >= total_len { return s; }
    padding := str_repeat(pad_char, (total_len - current) as i32);
    return padding + s;
}

fn str_pad_right(s: str, total_len: i64, pad_char: str) -> str {
    current := s.len();
    if current >= total_len { return s; }
    padding := str_repeat(pad_char, (total_len - current) as i32);
    return s + padding;
}
