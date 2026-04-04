// Error handling: Result enum + ? operator + error messages
//
// Ny uses tagged unions for error handling (like Rust),
// with string messages and automatic propagation via ?.

enum Result {
    Ok(i32),
    Err(str),
}

fn parse_int(s: str) -> Result {
    if s == "42" { return Result::Ok(42); }
    if s == "0" { return Result::Ok(0); }
    return Result::Err("invalid number: " + s);
}

fn divide(a: i32, b: i32) -> Result {
    if b == 0 {
        return Result::Err("division by zero");
    }
    return Result::Ok(a / b);
}

// ? operator: unwraps Ok or propagates Err automatically
fn compute(input: str) -> Result {
    value := parse_int(input)?;
    result := divide(84, value)?;
    return Result::Ok(result);
}

fn main() -> i32 {
    // Success path
    match compute("2") {
        Result::Ok(v) => println("success:", v),      // 42
        Result::Err(msg) => println("error:", msg),
    }

    // Error path — propagated from divide
    match compute("0") {
        Result::Ok(v) => println("unexpected:", v),
        Result::Err(msg) => println("error:", msg),    // "division by zero"
    }

    // Error path — propagated from parse_int
    match compute("abc") {
        Result::Ok(v) => println("unexpected:", v),
        Result::Err(msg) => println("error:", msg),    // "invalid number: abc"
    }

    // Rich errors with error_new/error_message
    code := error_new("something went wrong");
    println("message:", error_message(code));

    return 0;
}
