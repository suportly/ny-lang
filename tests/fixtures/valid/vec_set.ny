// Test: Vec.set(index, value) — index-based mutation

fn main() -> i32 {
    v :~ Vec<i32> = vec_new();
    v.push(10);
    v.push(20);
    v.push(30);

    // Overwrite element at index 1
    v.set(1, 99);

    a := v.get(0);  // 10
    b := v.get(1);  // 99 (was 20)
    c := v.get(2);  // 30

    // 10 + 99 + 30 = 139
    // 139 - 97 = 42
    return a + b + c - 97;
}
