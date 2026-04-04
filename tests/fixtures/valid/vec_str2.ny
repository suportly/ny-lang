// Test: Vec<str> — vector of strings with methods

fn main() -> i32 {
    v :~ Vec<str> = vec_new();
    v.push("hello");
    v.push("world");
    v.push("ny");

    first := v.get(0);
    last := v.get(2);

    // first.len()=5, last.len()=2, v.len()=3
    // Check string methods on retrieved elements
    ok :~ i32 = 0;
    if first.contains("ell") { ok += 10; }
    if last.starts_with("ny") { ok += 10; }

    // 5 + 2 + 3 + 20 + 12 = 42
    return first.len() as i32 + last.len() as i32 + v.len() as i32 + ok + 12;
}
