fn sum_array(a: [5]i32) -> i32 {
    total :~ i32 = 0;
    for i in 0..5 {
        total = total + a[i];
    }
    return total;
}

fn main() -> i32 {
    a : [5]i32 = [10, 20, 30, 40, 50];
    return sum_array(a);
}
