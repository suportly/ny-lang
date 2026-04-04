// Interactive Calculator — Demonstrates: read_line, str_to_int, loops, match
// A simple REPL-style calculator that reads expressions from stdin.
//
// Usage: ny run calculator.ny
// Then type: 10 + 5 <enter>
extern {
    fn putchar(c: i32) -> i32;
}

fn main() -> i32 {
    println("=== Ny Calculator ===");
    println("Enter: <num> <op> <num> (e.g. '10 + 5')");
    println("Type 'q' to quit");
    println("");
    loop {
        print("> ");
        line := read_line();
        if line.starts_with("q") {
            println("Goodbye!");
            break;
        }
        parts := str_split_count(line, " ");
        if parts != 3 {
            println("  Error: expected '<num> <op> <num>'");
            continue;
        }
        left_str := str_split_get(line, " ", 0);
        op := str_split_get(line, " ", 1);
        right_str := str_split_get(line, " ", 2);
        left := str_to_int(left_str);
        right := str_to_int(right_str);
        result :~ i32 = 0;
        valid :~ i32 = 1;
        if op.starts_with("+") {
            result = left + right;
        } else if op.starts_with("-") {
            result = left - right;
        } else if op.starts_with("*") {
            result = left * right;
        } else if op.starts_with("/") {
            if right == 0 {
                println("  Error: division by zero");
                valid = 0;
            } else {
                result = left / right;
            }
        } else {
            println("  Error: unknown operator (use + - * /)");
            valid = 0;
        }
        if valid == 1 {
            println("  = " + to_str(result));
        }
    }
    // Check for quit
    // Parse: split by space
    return 0;
}
