// Test: Vec<bool> — elem_size=1, verifies correct storage for boolean type

fn main() -> i32 {
    v :~ Vec<bool> = vec_new();

    v.push(true);
    v.push(false);
    v.push(true);

    len := v.len(); // 3

    a := v.get(0); // true
    b := v.get(1); // false
    c := v.get(2); // true

    // true(1) + false(0) + true(1) = 2, plus len(3) = 5
    // We use conditionals since bool arithmetic may not be directly supported
    result :~ i32 = 0;
    if a { result = result + 10; }
    if b { result = result + 10; }
    if c { result = result + 10; }

    // result = 20, len = 3, 20 + 3 = 23
    return result + len as i32;
}
