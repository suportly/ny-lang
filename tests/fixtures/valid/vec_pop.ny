// Test: Vec.pop() — remove and return last element

fn main() -> i32 {
    v :~ Vec<i32> = vec_new();
    v.push(10);
    v.push(20);
    v.push(30);

    // Pop last element
    last := v.pop();  // 30
    len := v.len();   // 2

    // Pop again
    second := v.pop();  // 20
    len2 := v.len();    // 1

    // last + second = 50, len + len2 = 3
    // 50 - 3 - 5 = 42
    return last + second - len as i32 - len2 as i32 - 5;
}
