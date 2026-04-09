extern "CUDA" {
    // This block is to test the parser's ability to handle GPU-specific FFI syntax.
    // The actual CUDA functions are not used in this test, as the focus is on
    // ensuring the `extern "CUDA"` syntax is correctly parsed and doesn't break
    // the compilation process.
    fn my_cuda_kernel(x: *mut f32, y: *f32, n: i32);
}

fn main(): int {
    // The test simply returns a value to confirm that the program compiled
    // and can be executed. The extern block is parsed but not linked.
    return 42;
}
