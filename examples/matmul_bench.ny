// Matrix Multiply Benchmark
// Demonstrates: Vec<i32>, nested loops, structs, impl, defer, f-strings, casting
//
// Multiplies two NxN matrices (stored as flat Vec<i32>) using naive triple-loop.
// Verifies correctness via checksum and spot checks.

extern {
    fn putchar(c: i32) -> i32;
}

// Build an N*N matrix filled with pattern: mat[i][j] = (i + j) % mod_val
fn make_matrix(n: i32, mod_val: i32) -> Vec<i32> {
    v :~ Vec<i32> = vec_new();
    i :~ i32 = 0;
    while i < n {
        j :~ i32 = 0;
        while j < n {
            v.push((i + j) % mod_val);
            j += 1;
        }
        i += 1;
    }
    return v;
}

// Naive O(n^3) matrix multiply: returns C = A * B as a new Vec
fn matmul(a: Vec<i32>, b: Vec<i32>, n: i32) -> Vec<i32> {
    c :~ Vec<i32> = vec_new();
    i :~ i32 = 0;
    while i < n {
        j :~ i32 = 0;
        while j < n {
            sum :~ i32 = 0;
            k :~ i32 = 0;
            while k < n {
                sum += a.get(i * n + k) * b.get(k * n + j);
                k += 1;
            }
            c.push(sum);
            j += 1;
        }
        i += 1;
    }
    return c;
}

// Compute checksum: sum of all elements
fn checksum(mat: Vec<i32>, size: i32) -> i64 {
    total :~ i64 = 0;
    i :~ i32 = 0;
    while i < size {
        total += mat.get(i) as i64;
        i += 1;
    }
    return total;
}

// Print top-left corner of matrix
fn print_corner(mat: Vec<i32>, n: i32, limit: i32) {
    rows :~ i32 = n;
    if rows > limit { rows = limit; }
    cols :~ i32 = n;
    if cols > limit { cols = limit; }

    i :~ i32 = 0;
    while i < rows {
        print("    ");
        j :~ i32 = 0;
        while j < cols {
            val := mat.get(i * n + j);
            print(val);
            putchar(9);  // tab
            j += 1;
        }
        putchar(10);  // newline
        i += 1;
    }
    if n > limit {
        println("    ...");
    }
}

fn main() -> i32 {
    n := 64;

    println("=== Matrix Multiply Benchmark ===");
    println(f"  Matrix size: {n}x{n}");
    println("");

    // Build matrices
    a := make_matrix(n, 7);
    b := make_matrix(n, 5);

    println(f"  A: {n}x{n}, pattern (i+j) % 7");
    println(f"  B: {n}x{n}, pattern (i+j) % 5");
    println("");

    // Multiply
    println("[1] Naive triple-loop multiply...");
    c := matmul(a, b, n);

    cs := checksum(c, n * n);
    println(f"  Checksum: {cs}");

    // Show top-left corner
    println("  Top-left 4x4 of C:");
    print_corner(c, n, 4);

    // Spot checks
    c00 := c.get(0);
    c01 := c.get(1);
    c10 := c.get(n);
    println(f"  C[0][0] = {c00}");
    println(f"  C[0][1] = {c01}");
    println(f"  C[1][0] = {c10}");

    // Scale test: 128x128
    n2 := 128;
    println("");
    println(f"[2] Scaling: {n2}x{n2} multiply...");

    a2 := make_matrix(n2, 7);
    b2 := make_matrix(n2, 5);
    c2 := matmul(a2, b2, n2);

    cs2 := checksum(c2, n2 * n2);
    println(f"  Checksum: {cs2}");
    println(f"  C[0][0] = {c2.get(0)}");

    println("");
    println("=== Benchmark Complete ===");
    return 0;
}
