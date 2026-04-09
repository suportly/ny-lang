use std::io;
use std::analytics;

pub fn main() -> i32 {
    // NOTE: This path is relative to the root of the project,
    // where the test runner will execute the compiled binary.
    let df = analytics::read_csv("tests/fixtures/data/test.csv");

    if df.columns.len() != 3 {
        return -1; // Wrong number of columns
    }

    let col_a = df.columns.get("col_a");
    let sum_a = col_a.sum(); // 1.0 + 2.5 + 3.0 + 4.5 - 1.0 = 10.0

    let col_b = df.columns.get("col_b");
    let mean_b = col_b.mean(); // (10.0 + 20.5 + 30.0 + 40.5 - 100.0) / 5 = 1.0 / 5 = 0.2

    let col_c = df.columns.get("col_c");
    let sum_c = col_c.sum(); // 100.0 + 200.5 + 300.0 + 400.5 - 1000.0 = 1.0
    
    // Combine results to return a single integer exit code for the test.
    // Floating point comparisons can be tricky, so we use integer arithmetic.
    // Expected: 10 + 1 + 2 = 13
    let result = (sum_a as i32) + (sum_c as i32) + ((mean_b * 10.0) as i32);

    return result;
}
