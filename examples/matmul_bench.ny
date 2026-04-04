// Matrix Multiply Benchmark — with timing
// Demonstrates: Vec<i32>, nested loops, clock_ms(), f-strings, structs
//
// Usage: ny run matmul_bench.ny
//    or: ny build matmul_bench.ny -O 2 -o matmul && ./matmul
// Matrix: flat Vec<i32>, row-major layout
fn mat_new(n: i32) -> Vec<i32> {
    v :~ Vec<i32> = vec_new();
    total := n * n;
    i :~ i32 = 0;
    while i < total {
        v.push(0);
        i += 1;
    }
    return v;
}

fn mat_init(mat: Vec<i32>, n: i32, mod_val: i32) {
    i :~ i32 = 0;
    while i < n {
        j :~ i32 = 0;
        while j < n {
            mat.set(i * n + j, i + j % mod_val);
            j += 1;
        }
        i += 1;
    }
}
// Naive O(n^3) matrix multiply: C = A * B (in-place)

fn matmul(a: Vec<i32>, b: Vec<i32>, c: Vec<i32>, n: i32) {
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
            c.set(i * n + j, sum);
            j += 1;
        }
        i += 1;
    }
}

fn checksum(mat: Vec<i32>, size: i32) -> i64 {
    total :~ i64 = 0;
    i :~ i32 = 0;
    while i < size {
        total += mat.get(i) as i64;
        i += 1;
    }
    return total;
}

fn bench(n: i32) {
    a := mat_new(n);
    b := mat_new(n);
    c := mat_new(n);
    mat_init(a, n, 7);
    mat_init(b, n, 5);
    start := clock_ms();
    matmul(a, b, c, n);
    elapsed := clock_ms() - start;
    cs := checksum(c, n * n);
    println("  " + to_str(n) + "x" + to_str(n) + ": " + to_str(elapsed) + "ms (checksum: " + to_str(cs) + ")");
}

fn main() -> i32 {
    println("=== Matrix Multiply Benchmark ===");
    println("");
    bench(32);
    bench(64);
    bench(128);
    bench(256);
    println("");
    println("=== Done ===");
    return 0;
}
