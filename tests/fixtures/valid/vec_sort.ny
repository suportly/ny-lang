// Test: Vec.sort() — in-place ascending sort

fn main() -> i32 {
    v :~ Vec<i32> = vec_new();
    v.push(5);
    v.push(3);
    v.push(8);
    v.push(1);
    v.push(4);

    v.sort();

    // After sort: [1, 3, 4, 5, 8]
    a := v.get(0);  // 1
    b := v.get(1);  // 3
    c := v.get(2);  // 4
    d := v.get(3);  // 5
    e := v.get(4);  // 8

    // Verify sorted order: 1 + 3 + 4 + 5 + 8 = 21
    // Also verify: a < b < c < d < e
    ok :~ i32 = 0;
    if a == 1 { ok = ok + 1; }
    if b == 3 { ok = ok + 1; }
    if c == 4 { ok = ok + 1; }
    if d == 5 { ok = ok + 1; }
    if e == 8 { ok = ok + 1; }

    // 21 + 5*ok = 21 + 25 = 46, - 4 = 42
    return a + b + c + d + e + ok * 5 - 4;
}
