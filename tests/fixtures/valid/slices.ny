// Phase 9: Slices - array range indexing, slice as function param, .len()

fn sum_slice(data: []i32, n: i64) -> i32 {
    total :~ i32 = 0;
    i :~ i32 = 0;
    while i as i64 < n {
        total += data[i];
        i += 1;
    }
    return total;
}

fn main() -> i32 {
    arr : [5]i32 = [10, 20, 30, 40, 50];

    // Create a slice from array: arr[1..4] = [20, 30, 40]
    s := arr[1..4];

    // Slice length
    len := s.len(); // 3

    // Slice indexing
    first := s[0];  // 20

    // Call function with slice param
    total := sum_slice(s, len);  // 20+30+40 = 90

    // total - first - len as i32 = 90 - 20 - 3 = 67
    // But we want something in 0-255 range
    // Let's use: total - first - len as i32 - 20 = 47
    return first + len as i32 + total - 90;  // 20 + 3 + 90 - 90 = 23
}
