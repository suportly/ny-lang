// error_trace — stack traces captured at error creation

enum Result {
    Ok(i32),
    Err(i32),
}

fn inner_fail() -> Result {
    return Result::Err(error_new("something went wrong"));
}

fn outer() -> Result {
    return inner_fail();
}

fn main() -> i32 {
    r := outer();
    match r {
        Result::Ok(v) => { return 1; },
        Result::Err(code) => {
            msg := error_message(code);
            if msg.len() < 5 { return 2; }

            trace := error_trace(code);
            // In debug mode, trace should contain function names
            // In release mode, trace is empty (traces skipped at -O2+)
            // Either way, it shouldn't crash
        },
    }
    return 42;
}
