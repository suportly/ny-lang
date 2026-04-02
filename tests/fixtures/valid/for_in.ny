// Test: for-in iteration over arrays

fn main() -> i32 {
    arr : [5]i32 = [10, 20, 30, 40, 50];

    total :~ i32 = 0;
    for item in arr {
        total += item;
    }

    // 10+20+30+40+50 = 150
    // Return 150 - 108 = 42
    return total - 108;
}
