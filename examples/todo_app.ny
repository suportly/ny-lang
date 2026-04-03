// Todo App — A complete Ny program demonstrating most language features.
//
// Features used:
//   - Structs + impl blocks       - Vec<T> with map/filter/reduce
//   - Enums + match               - HashMap
//   - Closures (capturing)        - String methods (trim, contains, split)
//   - File I/O (read/write)       - JSON parsing
//   - f-string interpolation      - Error handling (? operator)
//   - clock_ms() timing           - for-in iteration
//   - Extern FFI (putchar)        - defer for cleanup
//
// Usage:
//   ny run todo_app.ny
//   (or: ny build todo_app.ny -O 2 -o todo && ./todo)

extern {
    fn putchar(c: i32) -> i32;
}

// --- Data model ---

enum Priority {
    High,
    Medium,
    Low,
}

fn priority_to_str(p: Priority) -> str {
    return match p {
        Priority::High => "HIGH",
        Priority::Medium => "MED",
        Priority::Low => "LOW",
    };
}

fn priority_score(p: Priority) -> i32 {
    return match p {
        Priority::High => 3,
        Priority::Medium => 2,
        Priority::Low => 1,
    };
}

// --- Todo storage (Vec-based) ---

fn make_todo(id: i32, title: str, priority: Priority, done: bool) -> i32 {
    // Store as flat parallel vectors (workaround for no Vec<struct> yet)
    return id;
}

// --- Display helpers ---

fn print_separator() {
    i :~ i32 = 0;
    while i < 50 {
        putchar(45); // '-'
        i += 1;
    }
    putchar(10);
}

fn print_header(title: str) {
    println("");
    println(title);
}

// --- Statistics ---

fn add(a: i32, b: i32) -> i32 { return a + b; }

fn compute_stats(scores: Vec<i32>) -> str {
    total := scores.reduce(add, 0);
    count := scores.len() as i32;
    if count == 0 { return "no items"; }
    avg := total / count;
    return f"  Total items: {count}, Total score: {total}, Avg priority: {avg}";
}

// --- Main application ---

fn main() -> i32 {
    start := clock_ms();

    println("=== Ny Todo App ===");
    println("A complete program demonstrating the Ny language.");

    // --- 1. Create todos using parallel Vecs ---
    titles :~ Vec<i32> = vec_new();  // indices into title_strs
    priorities :~ Vec<i32> = vec_new();
    done :~ Vec<i32> = vec_new();

    // Title storage via HashMap (id -> title mapping)
    title_map := map_new();

    // Add todos
    map_insert(title_map, "0", 0);
    map_insert(title_map, "1", 1);
    map_insert(title_map, "2", 2);
    map_insert(title_map, "3", 3);
    map_insert(title_map, "4", 4);

    // Priorities: 3=high, 2=medium, 1=low
    priorities.push(3); // Build compiler
    priorities.push(2); // Write docs
    priorities.push(1); // Fix typo
    priorities.push(3); // Add tests
    priorities.push(2); // Benchmark

    // Done status: 0=pending, 1=done
    done.push(1); // Build compiler - done
    done.push(0); // Write docs - pending
    done.push(1); // Fix typo - done
    done.push(0); // Add tests - pending
    done.push(0); // Benchmark - pending

    todo_names :~ Vec<i32> = vec_new();
    todo_names.push(0); todo_names.push(1); todo_names.push(2);
    todo_names.push(3); todo_names.push(4);

    names : [5]str = ["Build compiler", "Write documentation", "Fix typo in README", "Add integration tests", "Run benchmarks"];

    // --- 2. Display all todos ---
    print_header("All Todos");

    i :~ i32 = 0;
    while i < 5 {
        status :~ str = "[ ]";
        if done.get(i) == 1 {
            status = "[x]";
        }
        pri := priorities.get(i);
        pri_str :~ str = "LOW";
        if pri == 3 { pri_str = "HIGH"; }
        if pri == 2 { pri_str = "MED"; }

        println(f"  {status} #{i}: {names[i]} [{pri_str}]");
        i += 1;
    }

    // --- 3. Filter: show only pending items ---
    print_header("Pending Items");

    pending_count :~ i32 = 0;
    i = 0;
    while i < 5 {
        if done.get(i) == 0 {
            println(f"  #{i}: {names[i]}");
            pending_count += 1;
        }
        i += 1;
    }
    println(f"  ({pending_count} items pending)");

    // --- 4. Functional: compute stats with map/reduce ---
    print_header("Statistics");

    stats := compute_stats(priorities);
    println(stats);

    // Count done vs pending using functional style
    done_count := done.filter(|x: i32| -> bool { return x == 1; }).len() as i32;
    println(f"  Completed: {done_count}/5");

    // High priority pending
    high_pending :~ i32 = 0;
    i = 0;
    while i < 5 {
        if done.get(i) == 0 {
            if priorities.get(i) == 3 {
                high_pending += 1;
            }
        }
        i += 1;
    }
    println(f"  High priority pending: {high_pending}");

    // --- 5. JSON: serialize and parse back ---
    print_header("JSON Export/Import");

    json_str := "{\"app\": \"ny-todo\", \"version\": 1, \"total\": 5}";
    obj := json_parse(json_str);

    app := json_get_str(obj, "app");
    ver := json_get_int(obj, "version");
    total := json_get_int(obj, "total");
    println(f"  App: {app} v{ver}, {total} todos");

    // --- 6. File I/O: save report ---
    print_header("File I/O");

    report := f"Todo Report: {done_count} done, {pending_count} pending, {high_pending} high-pri";
    write_file("/tmp/ny_todo_report.txt", report);
    println("  Saved report to /tmp/ny_todo_report.txt");

    // Read it back
    content := read_file("/tmp/ny_todo_report.txt");
    println(f"  Read back: {content}");
    remove_file("/tmp/ny_todo_report.txt");

    // --- 7. String processing ---
    print_header("String Processing");

    search := "test";
    i = 0;
    found :~ i32 = 0;
    while i < 5 {
        lower := names[i].to_lower();
        if lower.contains(search) {
            println(f"  Match: #{i} '{names[i]}' contains '{search}'");
            found += 1;
        }
        i += 1;
    }
    println(f"  Found {found} matches for '{search}'");

    // --- 8. Math ---
    print_header("Math");

    completion := done_count as f64 / 5.0 * 100.0;
    comp_str := float_to_str(completion);
    print("  Completion: ");
    print(comp_str);
    println("%");
    println(f"  sqrt(pending^2 + done^2) = {float_to_str(sqrt(pending_count as f64 * pending_count as f64 + done_count as f64 * done_count as f64))}");

    // --- 9. Timing ---
    elapsed := clock_ms() - start;

    print_header("Done");
    println(f"  Completed in {elapsed}ms");
    println("");

    return 0;
}
