// Ny Lang Benchmark Suite
// Demonstrates: generics, enums, match, structs, impl, for-in, Vec,
// modules, extern FFI, lambdas, defer, tagged unions, ? operator

use "math.ny";
use "sort.ny";

// Tagged union for error handling
enum Result {
    Ok(i32),
    Err(i32),
}

fn safe_div(a: i32, b: i32) -> Result {
    if b == 0 {
        return Result::Err(0);
    }
    return Result::Ok(a / b);
}

// Struct with impl block
struct Stats {
    sum: i32,
    count: i32,
    min_val: i32,
    max_val: i32,
}

impl Stats {
    fn average(self: Stats) -> i32 {
        if self.count == 0 { return 0; }
        return self.sum / self.count;
    }
}

fn compute_stats(arr: [10]i32) -> Stats {
    s :~ i32 = 0;
    lo :~ i32 = arr[0];
    hi :~ i32 = arr[0];

    for val in arr {
        s += val;
        if val < lo { lo = val; }
        if val > hi { hi = val; }
    }

    return Stats { sum: s, count: 10, min_val: lo, max_val: hi };
}

// Fibonacci (recursive, classic benchmark)
fn fibonacci(n: i32) -> i32 {
    if n <= 1 { return n; }
    return fibonacci(n - 1) + fibonacci(n - 2);
}

// Higher-order functions with lambdas
fn apply(f: fn(i32) -> i32, x: i32) -> i32 {
    return f(x);
}

// Extern FFI: use libc directly
extern {
    fn abs(x: i32) -> i32;
}

// Main: run all benchmarks
fn main() -> i32 {
    println("=== Ny Lang Benchmark Suite ===");
    println("");

    // 1. Fibonacci
    fib_result := fibonacci(20);
    print("fibonacci(20) = ");
    println(fib_result);

    // 2. Sorting
    data : [10]i32 = [64, 34, 25, 12, 22, 11, 90, 1, 45, 78];

    sorted := bubble_sort(data);
    print("bubble_sort sorted = ");
    println(is_sorted(sorted));

    sorted2 := selection_sort(data);
    print("selection_sort sorted = ");
    println(is_sorted(sorted2));

    sorted3 := insertion_sort(data);
    print("insertion_sort sorted = ");
    println(is_sorted(sorted3));

    // 3. Stats with structs + impl
    stats := compute_stats(data);
    print("sum = ");
    println(stats.sum);
    print("min = ");
    println(stats.min_val);
    print("max = ");
    println(stats.max_val);
    print("average = ");
    println(stats.average());

    // 4. Error handling with tagged unions + match
    div_result := safe_div(100, 4);
    val := match div_result {
        Result::Ok(v) => v,
        Result::Err(e) => e,
    };
    print("100 / 4 = ");
    println(val);

    // 5. Generics (monomorphization)
    bigger := max(42, 17);
    print("max(42, 17) = ");
    println(bigger);

    smaller := min(42, 17);
    print("min(42, 17) = ");
    println(smaller);

    // 6. GCD from imported module
    g := gcd(48, 18);
    print("gcd(48, 18) = ");
    println(g);

    // 7. Lambdas as first-class values
    double := |x: i32| -> i32 { return x * 2; };
    result := apply(double, 21);
    print("apply(double, 21) = ");
    println(result);

    // 8. Vec dynamic array
    v :~ Vec<i32> = vec_new();
    i :~ i32 = 0;
    while i < 10 {
        v.push(i * i);
        i += 1;
    }
    print("squares = ");
    println(v);
    print("vec.len() = ");
    println(v.len());

    // 9. Extern FFI
    abs_val := abs(-42);
    print("abs(-42) = ");
    println(abs_val);

    // 10. Heap allocation + defer
    buf := alloc(256);
    defer free(buf);

    // 11. String operations
    greeting := "Hello" + ", " + "Ny Lang!";
    println(greeting);
    print("greeting.len() = ");
    println(greeting.len());

    // 12. Slices
    arr : [5]i32 = [10, 20, 30, 40, 50];
    s := arr[1..4];
    print("slice [1..4] len = ");
    println(s.len());

    // 13. if let pattern matching
    check := safe_div(10, 2);
    if let Result::Ok(v) = check {
        print("if let Ok = ");
        println(v);
    }

    println("");
    println("=== All benchmarks complete ===");
    return 0;
}
