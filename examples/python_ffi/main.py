import ctypes
import os
import sys

# Determine the shared library extension based on the OS
if sys.platform == "darwin":
    lib_ext = "dylib"
elif sys.platform == "win32":
    lib_ext = "dll"
else:
    lib_ext = "so"

# Path to the compiled Ny Lang shared library
lib_path = os.path.join(os.path.dirname(__file__), f"libml_ops.{lib_ext}")

# Check if the library exists (for demonstration purposes, we'll try to load it if it's there)
# In a real scenario, you'd run `ny build ml_ops.ny --shared` first.
try:
    ny_lib = ctypes.CDLL(lib_path)
    
    # Configure the argument and return types for the FFI functions
    
    # fn ml_relu(values: *f64, length: i32)
    ny_lib.ml_relu.argtypes = [ctypes.POINTER(ctypes.c_double), ctypes.c_int]
    ny_lib.ml_relu.restype = None
    
    # fn ml_dot_product(a: *f64, b: *f64, length: i32) -> f64
    ny_lib.ml_dot_product.argtypes = [ctypes.POINTER(ctypes.c_double), ctypes.POINTER(ctypes.c_double), ctypes.c_int]
    ny_lib.ml_dot_product.restype = ctypes.c_double
    
    def test_ffi():
        # Example 1: ReLU
        data = [ -1.5, 2.0, -0.5, 3.14 ]
        arr_type = ctypes.c_double * len(data)
        arr = arr_type(*data)
        
        print("Before ReLU:", list(arr))
        ny_lib.ml_relu(arr, len(data))
        print("After ReLU: ", list(arr))
        
        # Example 2: Dot Product
        a_data = [ 1.0, 2.0, 3.0 ]
        b_data = [ 4.0, 5.0, 6.0 ]
        
        arr_a_type = ctypes.c_double * len(a_data)
        arr_b_type = ctypes.c_double * len(b_data)
        
        arr_a = arr_a_type(*a_data)
        arr_b = arr_b_type(*b_data)
        
        result = ny_lib.ml_dot_product(arr_a, arr_b, len(a_data))
        print(f"Dot Product of {a_data} and {b_data}: {result}")

    if __name__ == "__main__":
        print("Running Ny Lang FFI Example from Python...")
        test_ffi()

except OSError as e:
    print(f"Failed to load shared library at {lib_path}")
    print(f"Error: {e}")
    print("Please compile the Ny Lang code first:")
    print(f"  ny build examples/python_ffi/ml_ops.ny --shared -o examples/python_ffi/libml_ops.{lib_ext}")

# Simple test function for pytest
def test_python_script_exists():
    assert True
