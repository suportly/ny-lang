// Test: SIMD vector types — f32x4 arithmetic + reduce

fn main() -> i32 {
    a := simd_splat_f32x4(10.0);
    b := simd_splat_f32x4(0.5);

    // SIMD multiply: [10*0.5, 10*0.5, 10*0.5, 10*0.5] = [5, 5, 5, 5]
    c := a * b;

    // Horizontal sum: 5+5+5+5 = 20
    total := simd_reduce_add_f32(c);

    // Also test add
    d := a + a;   // [20, 20, 20, 20]
    d_sum := simd_reduce_add_f32(d);  // 80

    // total + d_sum/4 = 20 + 20 = 40, + 2 = 42
    return total as i32 + d_sum as i32 / 4 + 2;
}
