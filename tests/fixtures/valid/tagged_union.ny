// Phase 6: Data-carrying enums (tagged unions)

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

fn main() -> i32 {
    r := divide(84, 2);
    val := match r {
        Result::Ok(v) => v,
        Result::Err(e) => e,
    };
    return val; // 84 / 2 = 42
}
