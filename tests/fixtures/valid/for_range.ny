fn main() -> i32 {
    sum :~ i32 = 0;
    for i in 0..10 {
        sum = sum + i;
    }
    return sum;
}
