// Test: multiple closures with different captures — no corruption

fn main() -> i32 {
    a := 10;
    b := 20;
    c := 5;

    // Three closures capturing different values
    add_a := |x: i32| -> i32 { return x + a; };
    add_b := |x: i32| -> i32 { return x + b; };
    add_c := |x: i32| -> i32 { return x + c; };

    // Each closure should use its OWN captured value
    r1 := add_a(1);  // 1 + 10 = 11
    r2 := add_b(1);  // 1 + 20 = 21
    r3 := add_c(1);  // 1 + 5 = 6

    // Verify add_a still works after creating add_b and add_c
    r4 := add_a(2);  // 2 + 10 = 12

    // 11 + 21 + 6 + 12 = 50
    return r1 + r2 + r3 + r4;
}
