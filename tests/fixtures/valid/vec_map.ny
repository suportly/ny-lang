// Test: Vec.map() — apply function to each element

fn double(x: i32) -> i32 {
    return x * 2;
}

fn negate(x: i32) -> i32 {
    return 0 - x;
}

fn main() -> i32 {
    v :~ Vec<i32> = vec_new();
    v.push(1);
    v.push(2);
    v.push(3);

    // map with double: [2, 4, 6]
    doubled := v.map(double);
    a := doubled.get(0);  // 2
    b := doubled.get(1);  // 4
    c := doubled.get(2);  // 6

    // map with negate: [-1, -2, -3]
    negated := v.map(negate);
    d := negated.get(0);  // -1
    e := negated.get(1);  // -2
    f := negated.get(2);  // -3

    // 2+4+6 = 12, -1-2-3 = -6
    // 12 + (-6) = 6
    // Lengths: doubled.len()=3, negated.len()=3
    // 6 + 3 + 3 + 30 = 42
    return a + b + c + d + e + f + doubled.len() as i32 + negated.len() as i32 + 30;
}
