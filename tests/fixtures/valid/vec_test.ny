// Test: Vec<i32> dynamic array

fn main() -> i32 {
    v :~ Vec<i32> = vec_new();

    // Push values
    v.push(10);
    v.push(20);
    v.push(30);

    // Length check
    len := v.len();  // 3

    // Element access
    a := v.get(0);   // 10
    b := v.get(1);   // 20
    c := v.get(2);   // 30

    // a + b + c - len as i32 = 10 + 20 + 30 - 3 = 57
    return a + b + c - len as i32;
}
