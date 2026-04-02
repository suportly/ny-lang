// Test: closure with complex captures — field access, match, loops inside lambda

struct Point {
    x: i32,
    y: i32,
}

enum Dir {
    Up,
    Down,
}

fn main() -> i32 {
    p := Point { x: 10, y: 5 };
    scale := 2;

    // Closure that captures 'p' and 'scale', uses field access + math inside
    compute := |offset: i32| -> i32 {
        return p.x * scale + p.y + offset;
    };

    // compute(7) = 10*2 + 5 + 7 = 32
    a := compute(7);

    // Closure that captures a variable and uses it in a loop
    limit := 5;
    sum_to := |n: i32| -> i32 {
        total :~ i32 = 0;
        i :~ i32 = 0;
        while i < n {
            total += i;
            i += 1;
        }
        // Use captured 'limit' to offset
        return total - limit;
    };

    // sum_to(5) = (0+1+2+3+4) - 5 = 10 - 5 = 5
    b := sum_to(5);

    // Closure that captures and uses conditional
    threshold := 15;
    check := |val: i32| -> i32 {
        if val > threshold {
            return 1;
        }
        return 0;
    };

    // check(20) = 1 (20 > 15), check(10) = 0 (10 <= 15)
    c := check(20);
    d := check(10);

    // a + b + c + d = 32 + 5 + 1 + 0 = 38
    // +4 to get 42
    return a + b + c + d + 4;
}
