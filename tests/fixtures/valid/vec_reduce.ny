// Test: Vec.reduce(fn, init) + Vec.for_each(fn)

fn add(a: i32, b: i32) -> i32 { return a + b; }
fn mul(a: i32, b: i32) -> i32 { return a * b; }
fn max(a: i32, b: i32) -> i32 { if a > b { return a; } return b; }

fn main() -> i32 {
    v :~ Vec<i32> = vec_new();
    v.push(3); v.push(7); v.push(1); v.push(9); v.push(2);

    sum := v.reduce(add, 0);       // 3+7+1+9+2 = 22
    product := v.reduce(mul, 1);   // 3*7*1*9*2 = 378
    maximum := v.reduce(max, 0);   // 9

    // 22 + 9 = 31, need +11 = 42
    // product % 100 = 78, 78 - 67 = 11
    return sum + maximum + product % 100 - 67;
}
