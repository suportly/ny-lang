// Phase 10: File I/O

fn main() -> i32 {
    // Write a file
    fp := fopen("/tmp/ny_test_io.txt\0", "w\0");
    fwrite_str(fp, "hello from ny\0");
    fclose(fp);

    // Read it back
    fp2 := fopen("/tmp/ny_test_io.txt\0", "r\0");
    first_byte := fread_byte(fp2);
    fclose(fp2);

    // 'h' = 104, we want exit code in 0-255 range
    // 104 - 62 = 42
    return first_byte - 62;
}
