// Test: pointer arithmetic

fn main() -> i32 {
    buf := alloc(32);
    defer free(buf);

    // Write bytes at offsets
    *buf = 10 as u8;
    p1 := buf + 1;
    *p1 = 20 as u8;
    p2 := buf + 2;
    *p2 = 12 as u8;

    // Read them back
    a := *buf;
    b := *(buf + 1);
    c := *(buf + 2);

    return a as i32 + b as i32 + c as i32;  // 10 + 20 + 12 = 42
}
