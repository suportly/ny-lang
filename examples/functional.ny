// Functional Programming Demo — map, filter, reduce on Vec<T>
// Shows: higher-order functions, function composition, data pipelines
//
// Usage: ny run functional.ny

fn square(x: i32) -> i32 { return x * x; }
fn double(x: i32) -> i32 { return x * 2; }
fn add(a: i32, b: i32) -> i32 { return a + b; }
fn is_even(x: i32) -> bool { return x % 2 == 0; }
fn is_positive(x: i32) -> bool { return x > 0; }

fn print_item(x: i32) {
    print("  ");
    println(x);
}

fn main() -> i32 {
    // Build a range [1..10]
    nums :~ Vec<i32> = vec_new();
    i :~ i32 = 1;
    while i <= 10 {
        nums.push(i);
        i += 1;
    }

    println("=== Functional Programming Demo ===");
    println("");

    // 1. map: square each number
    println("[1] Squares:");
    squares := nums.map(square);
    println(squares);

    // 2. filter: keep even numbers
    println("");
    println("[2] Even numbers:");
    evens := nums.filter(is_even);
    println(evens);

    // 3. reduce: sum all numbers
    println("");
    total := nums.reduce(add, 0);
    println(f"[3] Sum of 1..10 = {total}");

    // 4. Pipeline: square → filter even → sum
    println("");
    pipeline := nums.map(square).filter(is_even).reduce(add, 0);
    println(f"[4] Sum of even squares = {pipeline}");

    // 5. for_each: print each
    println("");
    println("[5] Items:");
    nums.filter(is_even).for_each(print_item);

    println("");
    println("=== Done ===");
    return 0;
}
