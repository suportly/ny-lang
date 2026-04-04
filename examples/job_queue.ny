// ============================================================================
// Job Queue — a complete Ny program showcasing all Go-style features
//
// A concurrent job processing system with multiple workers, typed channels,
// dynamic dispatch, error handling, and analytics.
//
// Features demonstrated:
//   struct, impl, enum, match, interface, dyn Trait, new, go, chan<T>,
//   select, ?T, ??, if let, nil, var, type, Result, ?, error_new,
//   error_message, for k/v in map, println, f-string, defer, Vec,
//   HashMap, map/filter/reduce, clock_ms
//
// Usage: ny run examples/job_queue.ny
// ============================================================================

// ---- Type aliases (used in variable declarations) ----

type JobID = i32;
type WorkerID = i32;

// ---- Enums ----

enum Result {
    Ok(i32),
    Err(str),
}

// ---- Structs ----

struct Job {
    id: i32,
    payload: i32,
}

struct Stats {
    succeeded: i32,
    failed: i32,
    total_value: i32,
}

// ---- Interface (trait with Go naming) ----

interface Handler {
    fn handle(self: i32, payload: i32) -> i32;
}

// ---- Handler implementations ----

struct Multiplier { factor: i32 }

impl Handler for Multiplier {
    fn handle(self: Multiplier, payload: i32) -> i32 {
        return self.factor * payload;
    }
}

struct Validator { max_value: i32 }

impl Handler for Validator {
    fn handle(self: Validator, payload: i32) -> i32 {
        if payload > self.max_value {
            return -1;  // signal failure
        }
        return payload * 2;
    }
}

// ---- Factory: returns dyn Handler ----

fn make_handler(kind: i32) -> dyn Handler {
    if kind == 0 {
        return new Multiplier { factor: 10 };
    }
    return new Validator { max_value: 50 };
}

// ---- Worker function (runs as goroutine) ----

fn worker(id: WorkerID, handler_kind: i32, job_ch: *u8, result_ch: *u8) {
    // Create handler via factory (dyn dispatch)
    handler := make_handler(handler_kind);

    // Process jobs from channel
    var count = 0;
    for i in 0..5 {
        payload := channel_recv(job_ch);
        if payload == 0 { break; }

        // Dynamic dispatch: calls Multiplier.handle or Validator.handle
        result := handler.handle(payload);
        channel_send(result_ch, result);
        count = count + 1;
    }
    println(f"  Worker {id}: processed {count} jobs");
}

// ---- Process results with error handling ----

fn process_result(value: i32) -> Result {
    if value < 0 {
        return Result::Err(error_message(error_new("job failed: value exceeded limit")));
    }
    return Result::Ok(value);
}

fn unwrap_or_log(r: Result) -> i32 {
    return match r {
        Result::Ok(v) => v,
        Result::Err(msg) => {
            println("  ERROR:", msg);
            0
        },
    };
}

// ---- Optional pointer demo ----

struct Config {
    workers: i32,
    jobs: i32,
}

fn find_config(use_custom: bool) -> ?*Config {
    if use_custom {
        return new Config { workers: 3, jobs: 9 };
    }
    return nil;
}

// ---- Main ----

fn main() -> i32 {
    println("=== Job Queue Demo ===");
    start := clock_ms();

    // Optional types: ?T + ?? null coalescing
    custom := find_config(true);
    default_cfg := new Config { workers: 2, jobs: 6 };
    cfg := custom ?? default_cfg;

    // if let: safe unwrap of optional
    if let c = find_config(false) {
        println("custom config found (unexpected)");
    } else {
        println("  Using default-or-custom config");
    }

    num_workers := cfg.workers;
    num_jobs := cfg.jobs;
    println(f"  Workers: {num_workers}, Jobs: {num_jobs}");

    // Create channels
    job_ch := channel_new(32);
    result_ch := channel_new(32);

    // Spawn workers as goroutines
    println("  Spawning workers...");
    for w in 0..num_workers {
        // Alternate handler types: Multiplier (0) and Validator (1)
        handler_kind := w % 2;
        go worker(w, handler_kind, job_ch, result_ch);
    }

    // Submit jobs
    println("  Submitting jobs...");
    for j in 0..num_jobs {
        payload := (j + 1) * 10;  // 10, 20, 30, ... 90
        channel_send(job_ch, payload);
    }

    // Send shutdown signals (0 = stop)
    for w in 0..num_workers {
        channel_send(job_ch, 0);
    }

    // Collect results with error handling
    var results : Vec<i32> = vec_new();
    var succeeded = 0;
    var failed = 0;

    for j in 0..num_jobs {
        raw := channel_recv(result_ch);
        r := process_result(raw);
        value := unwrap_or_log(r);
        results.push(value);
        if raw >= 0 {
            succeeded = succeeded + 1;
        } else {
            failed = failed + 1;
        }
    }

    channel_close(job_ch);
    channel_close(result_ch);

    // ---- Analytics with functional operations ----

    // Sum of successful results
    total := results.reduce(|a: i32, b: i32| -> i32 { return a + b; }, 0);

    // Count results > 100
    big_results := results.filter(|x: i32| -> bool { return x > 100; });

    // HashMap for tracking per-worker stats
    stats := map_new();
    map_insert(stats, "succeeded", succeeded);
    map_insert(stats, "failed", failed);
    map_insert(stats, "total_value", total);

    // ---- Output ----

    elapsed := clock_ms() - start;
    println("");
    println("=== Results ===");

    // for key, value in map — Go-style iteration
    for key, value in stats {
        println(f"  {key}: {value}");
    }

    println(f"  Big results (>100): {big_results.len()}");
    println(f"  Time: {elapsed}ms");
    println("=== Done ===");

    map_free(stats);
    return 0;
}
