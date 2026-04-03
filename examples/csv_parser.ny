// CSV Parser — Demonstrates: string split, HashMap, f-strings
// Parses CSV data and computes per-column statistics.
//
// Usage: ny run csv_parser.ny

fn main() -> i32 {
    data := "Alice,95,A|Bob,82,B|Carol,91,A|Dave,78,C|Eve,88,B";

    num_rows := str_split_count(data, "|");
    println("=== CSV Parser Demo ===");
    println(f"  Rows: {num_rows}");
    println("");

    total :~ i32 = 0;
    min_score :~ i32 = 999;
    max_score :~ i32 = 0;
    grades := map_new();

    i :~ i32 = 0;
    while i < num_rows {
        row := str_split_get(data, "|", i);
        name := str_split_get(row, ",", 0);
        score_str := str_split_get(row, ",", 1);
        grade := str_split_get(row, ",", 2);
        score := str_to_int(score_str);

        total += score;
        if score < min_score { min_score = score; }
        if score > max_score { max_score = score; }

        prev := map_get(grades, grade);
        map_insert(grades, grade, prev + 1);

        // Print each row
        print("  ");
        print(name);
        print(": score=");
        print(score);
        print(", grade=");
        println(grade);

        i += 1;
    }

    avg := total / num_rows;

    println("");
    println(f"  Students: {num_rows}");
    println(f"  Total: {total}, Average: {avg}");
    println(f"  Min: {min_score}, Max: {max_score}");
    println("");
    println("  Grade distribution:");

    if map_contains(grades, "A") {
        print("    A: ");
        println(map_get(grades, "A"));
    }
    if map_contains(grades, "B") {
        print("    B: ");
        println(map_get(grades, "B"));
    }
    if map_contains(grades, "C") {
        print("    C: ");
        println(map_get(grades, "C"));
    }

    println("");
    println("=== Done ===");
    return 0;
}
