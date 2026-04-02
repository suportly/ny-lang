// Phase 6: loop keyword

fn main() -> i32 {
    sum :~ i32 = 0;
    i :~ i32 = 0;

    loop {
        if i >= 10 {
            break;
        }
        sum += i;
        i += 1;
    }

    return sum; // 0+1+2+...+9 = 45
}
