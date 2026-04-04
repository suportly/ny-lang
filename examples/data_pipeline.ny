// Data Pipeline — Real-world example combining multiple features
// Reads JSON config, processes numerical data, writes results to file
//
// Features: JSON, File I/O, Vec, map/filter/reduce, math, structs, f-strings

fn add(a: i32, b: i32) -> i32 { return a + b; }
fn square(x: i32) -> i32 { return x * x; }
fn is_positive(x: i32) -> bool { return x > 0; }

fn std_dev(values: Vec<i32>, mean: f64) -> f64 {
    sum_sq :~ f64 = 0.0;
    i :~ i32 = 0;
    n := values.len() as i32;
    while i < n {
        diff := values.get(i) as f64 - mean;
        sum_sq += diff * diff;
        i += 1;
    }
    return sqrt(sum_sq / n as f64);
}

fn main() -> i32 {
    start := clock_ms();

    println("=== Ny Data Pipeline ===");
    println("");

    // 1. Parse config from JSON
    config := json_parse("{\"name\": \"sensor-data\", \"samples\": 1000, \"threshold\": 50}");
    defer json_free(config);

    name := json_get_str(config, "name");
    samples := json_get_int(config, "samples");
    threshold := json_get_int(config, "threshold");
    println(f"Config: {name}, {samples} samples, threshold={threshold}");

    // 2. Generate synthetic sensor data (sine wave + noise)
    data :~ Vec<i32> = vec_new();
    i :~ i32 = 0;
    while i < samples {
        val := sin(i as f64 * 0.1) * 100.0;
        noise := (i * 17 + 31) % 41 - 20;  // deterministic "noise"
        data.push(val as i32 + noise);
        i += 1;
    }
    println(f"Generated {data.len()} samples");

    // 3. Filter: keep values above threshold
    above := data.filter(|x: i32| -> bool { return x > 50; });
    below := data.filter(|x: i32| -> bool { return x < -50; });
    println(f"Above {threshold}: {above.len()} samples");
    println(f"Below -{threshold}: {below.len()} samples");

    // 4. Statistics using functional methods
    total := data.reduce(add, 0);
    mean := total as f64 / data.len() as f64;
    sd := std_dev(data, mean);

    // Squares of above-threshold values
    squares := above.map(square);
    sq_sum := squares.sum();

    println("");
    println("Statistics:");
    println(f"  Mean: {float_to_str(mean)}");
    println(f"  Std Dev: {float_to_str(sd)}");
    println(f"  Sum of squares (above threshold): {sq_sum}");

    // 5. Check conditions
    has_extreme := data.any(|x: i32| -> bool { return x > 90; });
    all_bounded := data.all(|x: i32| -> bool { return x < 200; });
    print("  Has extreme values (>90): ");
    println(has_extreme);
    print("  All bounded (<200): ");
    println(all_bounded);

    // 6. Write results to file
    report := f"Pipeline: {name}\nSamples: {samples}\nMean: {float_to_str(mean)}\nStdDev: {float_to_str(sd)}\nAbove threshold: {above.len()}\n";
    write_file("/tmp/ny_pipeline_report.txt", report);
    println("");
    println("Report written to /tmp/ny_pipeline_report.txt");

    // 7. Read back and verify
    content := read_file("/tmp/ny_pipeline_report.txt");
    println(f"Verified: {content.len()} bytes written");
    remove_file("/tmp/ny_pipeline_report.txt");

    elapsed := clock_ms() - start;
    println(f"\nCompleted in {elapsed}ms");
    return 0;
}
