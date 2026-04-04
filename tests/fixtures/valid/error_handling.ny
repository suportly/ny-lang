// Phase 34: Error handling with string messages
// Tests: error_new, error_message, ? operator with rich errors

enum Result {
    Ok(i32),
    Err(i32),
}

fn divide(a: i32, b: i32) -> Result {
    if b == 0 {
        return Result::Err(error_new("division by zero"));
    }
    return Result::Ok(a / b);
}

fn compute() -> Result {
    x := divide(84, 2)?;
    y := divide(x, 0)?;   // this will fail
    return Result::Ok(y);
}

fn main() -> i32 {
    // Successful case
    r1 := divide(84, 2);
    v1 := match r1 {
        Result::Ok(v) => v,
        Result::Err(e) => 0,
    };
    if v1 != 42 { return 1; }

    // Error case with message
    r2 := divide(10, 0);
    match r2 {
        Result::Ok(v) => { return 2; },
        Result::Err(code) => {
            msg := error_message(code);
            // msg should be "division by zero"
            if msg.len() < 10 { return 3; }
        },
    }

    // ? operator propagates errors
    r3 := compute();
    match r3 {
        Result::Ok(v) => { return 4; },
        Result::Err(code) => {
            msg := error_message(code);
            if msg.len() < 5 { return 5; }
        },
    }

    return 42;
}
