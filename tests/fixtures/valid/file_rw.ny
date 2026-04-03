// Test: read_file / write_file builtins (no null terminators needed)

fn main() -> i32 {
    // Write
    write_file("/tmp/ny_test_rw2.txt", "hello world from ny");

    // Read back
    content := read_file("/tmp/ny_test_rw2.txt");

    // Verify length: "hello world from ny" = 19
    len := content.len();

    // Verify content
    ok :~ i32 = 0;
    if content.starts_with("hello") { ok += 10; }
    if content.ends_with("ny") { ok += 10; }

    // 19 + 10 + 10 - len(19) + 32 = 42
    return ok + len as i32 + 3;
}
