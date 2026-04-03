// Test: Vec.any(pred) and Vec.all(pred)

fn is_even(x: i32) -> bool { return x % 2 == 0; }
fn is_positive(x: i32) -> bool { return x > 0; }

fn main() -> i32 {
    evens :~ Vec<i32> = vec_new();
    evens.push(2); evens.push(4); evens.push(6);

    mixed :~ Vec<i32> = vec_new();
    mixed.push(1); mixed.push(2); mixed.push(3);

    result :~ i32 = 0;

    if evens.all(is_even) { result += 10; }     // true
    if evens.any(is_positive) { result += 10; }  // true
    if !mixed.all(is_even) { result += 10; }     // true (not all even)
    if mixed.any(is_even) { result += 10; }      // true (2 is even)
    if mixed.all(is_positive) { result += 2; }   // true (all > 0)

    // 10 + 10 + 10 + 10 + 2 = 42
    return result;
}
