// Test: Vec<f64> — vector of floats

fn main() -> i32 {
    v :~ Vec<f64> = vec_new();
    v.push(1.5);
    v.push(2.5);
    v.push(3.0);

    len := v.len();  // 3

    a := v.get(0);   // 1.5
    b := v.get(1);   // 2.5
    c := v.get(2);   // 3.0

    // sum = 1.5 + 2.5 + 3.0 = 7.0
    sum := a + b + c;

    // 7.0 as i32 = 7, plus len 3 = 10
    return sum as i32 + len as i32;
}
