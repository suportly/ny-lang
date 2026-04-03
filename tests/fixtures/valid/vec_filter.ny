// Test: Vec.filter(predicate) — keep elements where fn returns true

fn is_positive(x: i32) -> bool {
    return x > 0;
}

fn is_big(x: i32) -> bool {
    return x > 5;
}

fn main() -> i32 {
    v :~ Vec<i32> = vec_new();
    v.push(-3);
    v.push(1);
    v.push(-1);
    v.push(7);
    v.push(0);
    v.push(10);

    // Filter positive: [1, 7, 10]
    pos := v.filter(is_positive);
    a := pos.len();  // 3

    // Filter big: [7, 10]
    big := v.filter(is_big);
    b := big.len();  // 2

    // Chain: filter then get
    sum :~ i32 = 0;
    i :~ i32 = 0;
    while i as i64 < pos.len() {
        sum += pos.get(i);
        i += 1;
    }

    // sum = 1+7+10 = 18, a=3, b=2
    // 18 + 3 + 2 + 19 = 42
    return sum + a as i32 + b as i32 + 19;
}
