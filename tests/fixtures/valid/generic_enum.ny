// Test: generic enums — Option<T> and Result<T, E>

enum Option<T> {
    Some(T),
    None,
}

enum Result<T, E> {
    Ok(T),
    Err(E),
}

fn find(arr: [5]i32, target: i32) -> Option<i32> {
    i :~ i32 = 0;
    while i < 5 {
        if arr[i] == target {
            return Option_i32::Some(arr[i]);
        }
        i += 1;
    }
    return Option_i32::None;
}

fn safe_div(a: i32, b: i32) -> Result<i32, i32> {
    if b == 0 {
        return Result_i32_i32::Err(0);
    }
    return Result_i32_i32::Ok(a / b);
}

fn main() -> i32 {
    arr : [5]i32 = [10, 20, 30, 40, 50];

    // Find 30 in array
    result := find(arr, 30);
    val := match result {
        Option_i32::Some(v) => v,
        Option_i32::None => 0,
    };

    // Safe division
    div := safe_div(24, 2);
    d := match div {
        Result_i32_i32::Ok(v) => v,
        Result_i32_i32::Err(e) => e,
    };

    // 30 + 12 = 42
    return val + d;
}
