// Result with string error messages — mixed-size enum payloads
// Ok(i32) = 4 bytes, Err(str) = 16 bytes — tests union layout

enum Result {
    Ok(i32),
    Err(str),
}

fn divide(a: i32, b: i32) -> Result {
    if b == 0 {
        return Result::Err("division by zero");
    }
    return Result::Ok(a / b);
}

fn main() -> i32 {
    // Success case
    r1 := divide(84, 2);
    v1 := match r1 {
        Result::Ok(v) => v,
        Result::Err(msg) => 0,
    };
    if v1 != 42 { return 1; }

    // Error case — str payload
    r2 := divide(10, 0);
    match r2 {
        Result::Ok(v) => { return 2; },
        Result::Err(msg) => {
            if msg.len() < 5 { return 3; }
        },
    }

    return 42;
}
