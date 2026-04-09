// tests/ffi_tests.rs

use ny_lang::ffi;

#[test]
fn test_get_ffi_backend() {
    // Test CUDA backend
    let cuda_backend = ffi::get_ffi_backend("cuda");
    assert!(cuda_backend.is_some());
    assert_eq!(cuda_backend.unwrap()->name(), "cuda");

    // Test OpenCL backend
    let opencl_backend = ffi::get_ffi_backend("opencl");
    assert!(opencl_backend.is_some());
    assert_eq!(opencl_backend.unwrap()->name(), "opencl");

    // Test invalid backend
    let invalid_backend = ffi::get_ffi_backend("invalid");
    assert!(invalid_backend.is_none());
}

#[test]
fn test_cuda_ffi_bindings() {
    let cuda_backend = ffi::get_ffi_backend("cuda").unwrap();
    let bindings = cuda_backend.generate_bindings().unwrap();
    assert!(bindings.contains("CUDA Bindings (Placeholder)"));
    assert!(bindings.contains("cuInit"));
}

#[test]
fn test_opencl_ffi_bindings() {
    let opencl_backend = ffi::get_ffi_backend("opencl").unwrap();
    let bindings = opencl_backend.generate_bindings().unwrap();
    assert!(bindings.contains("OpenCL Bindings (Placeholder)"));
    assert!(bindings.contains("clGetPlatformIDs"));
}
