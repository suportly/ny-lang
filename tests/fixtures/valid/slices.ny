// Phase 9: Slices - array range indexing and slice operations

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
    second := s[1]; // 30

    // first + second - len as i32 = 20 + 30 - 3 = 47
    // We want exit code 47
    return first + second - len as i32;
}
