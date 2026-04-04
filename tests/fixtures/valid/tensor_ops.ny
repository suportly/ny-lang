// Test: Tensor<f64> — matrix operations

fn main() -> i32 {
    a := tensor_zeros(2, 3);
    defer tensor_free(a);

    // Set values: [[1,2,3],[4,5,6]]
    tensor_set(a, 0, 0, 1.0);
    tensor_set(a, 0, 1, 2.0);
    tensor_set(a, 0, 2, 3.0);
    tensor_set(a, 1, 0, 4.0);
    tensor_set(a, 1, 1, 5.0);
    tensor_set(a, 1, 2, 6.0);

    // Get
    v := tensor_get(a, 1, 2);  // 6.0

    // Shape
    rows := tensor_rows(a);  // 2
    cols := tensor_cols(a);  // 3

    // Sum
    sum := tensor_sum(a);  // 21.0

    // Matmul: (2x3) @ (3x2) → (2x2)
    b := tensor_transpose(a);  // (3x2)
    defer tensor_free(b);
    c := tensor_matmul(a, b);  // (2x2)
    defer tensor_free(c);

    // c[0][0] = 1*1+2*2+3*3 = 14
    c00 := tensor_get(c, 0, 0);  // 14.0

    // v=6, rows=2, cols=3, sum=21, c00=14
    // 6 + 2 + 3 + 21 - 14 + 24 = 42
    return v as i32 + rows as i32 + cols as i32 + sum as i32 - c00 as i32 + 24;
}
