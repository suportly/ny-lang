fn main() -> i32 {
    x :~ i32 = 0;
    for i in 0..10 {
        x += i;
    }
    return x;
}
