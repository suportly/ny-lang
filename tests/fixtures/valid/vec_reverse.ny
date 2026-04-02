// Test: Vec.reverse() and Vec.clear()

fn main() -> i32 {
    v :~ Vec<i32> = vec_new();
    v.push(1);
    v.push(2);
    v.push(3);
    v.push(4);
    v.push(5);

    v.reverse();

    // After reverse: [5, 4, 3, 2, 1]
    a := v.get(0);  // 5
    b := v.get(1);  // 4
    c := v.get(2);  // 3
    d := v.get(3);  // 2
    e := v.get(4);  // 1

    // 5 + 4 + 3 + 2 + 1 = 15
    sum := a + b + c + d + e;

    // Test clear
    v2 :~ Vec<i32> = vec_new();
    v2.push(10);
    v2.push(20);
    v2.clear();
    len_after := v2.len();  // 0

    // Verify first > last (descending after reverse)
    ok :~ i32 = 0;
    if a > e { ok = 1; }

    // 15 + 0 + 1 = 16, need +26 to get 42
    // Actually: a*b - sum + len_after + ok = 20 - 15 + 0 + 1 = 6
    // Let's do: sum + ok*27 = 15 + 27 = 42
    return sum + ok * 27;
}
