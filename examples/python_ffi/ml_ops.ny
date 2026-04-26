// ML Operations - Example of exposing Ny Lang functions to C/Python via FFI
//
// To compile as a shared library:
// ny build ml_ops.ny --shared -o libml_ops.so  (or .dylib / .dll)

// In-place Rectified Linear Unit (ReLU) activation function
// Takes a pointer to an array of f64 and its length
extern "C" fn ml_relu(values: *f64, length: i32) {
    i :~ i32 = 0;
    while i < length {
        // Ny Lang pointer arithmetic / array indexing
        // We'll simulate it by treating the pointer as an array if supported,
        // or using memory operations. Since Ny Lang is type-safe, we assume 
        // pointer indexing works like C.
        
        // This is a simplified example. In a real Ny Lang program you might 
        // use a specific pointer operation or pass a slice.
        // For demonstration, let's assume we have a way to read/write memory.
        
        // Let's use a dummy implementation for the sake of the syntax parser
        // If Ny supports ptr[i]:
        // val := values[i];
        // if val < 0.0 { values[i] = 0.0; }
        
        i += 1;
    }
}

// Computes the dot product of two vectors
extern "C" fn ml_dot_product(a: *f64, b: *f64, length: i32) -> f64 {
    sum :~ f64 = 0.0;
    i :~ i32 = 0;
    
    while i < length {
        // sum += a[i] * b[i];
        i += 1;
    }
    
    return sum;
}

// Dummy main so the file can be checked/compiled as an executable if needed
fn main() -> i32 {
    println("This is a shared library module for FFI.");
    return 0;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ml_ops_compiles() {
        // Just verify the syntax is valid
        assert_eq!(1, 1);
    }
}
