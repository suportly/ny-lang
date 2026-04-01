// Test: ? operator on tagged unions

enum Result {
    Ok(i32),
    Err(i32),
}

fn divide(a: i32, b: i32) -> Result {
    if b == 0 {
        return Result::Err(0);
    }
    return Result::Ok(a / b);
}

fn compute() -> Result {
    // Use ? to extract Ok value or propagate Err
    x := divide(84, 2)?;   // Ok(42) → x = 42
    y := divide(x, 1)?;    // Ok(42) → y = 42
    return Result::Ok(y);
}

fn main() -> i32 {
    r := compute();
    return match r {
        Result::Ok(v) => v,
        Result::Err(e) => e,
    };
}
