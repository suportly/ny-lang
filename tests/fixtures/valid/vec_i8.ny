// Test: Vec<i8> — elem_size=1, verifies correct storage for small types

fn main() -> i32 {
    v :~ Vec<i8> = vec_new();

    // Push i8 values
    v.push(10 as i8);
    v.push(20 as i8);
    v.push(12 as i8);

    // Length check
    len := v.len(); // 3

    // Element access
    a := v.get(0); // 10
    b := v.get(1); // 20
    c := v.get(2); // 12

    // 10 + 20 + 12 = 42
    return (a + b + c) as i32;
}
