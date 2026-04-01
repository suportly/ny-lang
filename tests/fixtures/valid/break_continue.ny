fn main() -> i32 {
    sum :~ i32 = 0;
    for i in 0..100 {
        if i == 10 {
            break;
        }
        if i % 2 != 0 {
            continue;
        }
        sum = sum + i;
    }
    return sum;
}
