fn swap(a: *i32, b: *i32) -> i32 {
    tmp := *a;
    *a = *b;
    *b = tmp;
    return 0;
}

fn main() -> i32 {
    x :~ i32 = 10;
    y :~ i32 = 20;
    swap(&x, &y);
    return x;
}
