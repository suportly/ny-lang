// Test: for-in iteration over Vec<i32>

fn main() -> i32 {
    v :~ Vec<i32> = vec_new();
    v.push(5);
    v.push(10);
    v.push(15);
    v.push(12);

    total :~ i32 = 0;
    for item in v {
        total += item;
    }

    // 5 + 10 + 15 + 12 = 42
    return total;
}
