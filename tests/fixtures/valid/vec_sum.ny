// Test: Vec.sum() — shorthand for summing all elements

fn main() -> i32 {
    v :~ Vec<i32> = vec_new();
    v.push(5); v.push(10); v.push(15); v.push(12);

    total := v.sum();  // 5+10+15+12 = 42
    return total;
}
