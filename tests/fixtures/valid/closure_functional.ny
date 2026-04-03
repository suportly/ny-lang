// Test: capturing closures with map and filter

fn main() -> i32 {
    v :~ Vec<i32> = vec_new();
    v.push(1); v.push(2); v.push(3); v.push(4); v.push(5);

    // Inline capturing closure with map
    factor := 10;
    scaled := v.map(|x: i32| -> i32 { return x * factor; });
    // [10, 20, 30, 40, 50]

    // Inline capturing closure with filter
    threshold := 25;
    big := scaled.filter(|x: i32| -> bool { return x > threshold; });
    // [30, 40, 50]

    // big.get(0)=30, big.len()=3, scaled.len()=5
    // 30 + 3 + 5 + 4 = 42
    return big.get(0) + big.len() as i32 + scaled.len() as i32 + 4;
}
