// stdlib/io.ny — I/O utility functions

fn print_line(msg: str) {
    println(msg);
}

fn print_int(label: str, value: i32) {
    print(label);
    println(value);
}

fn print_bool(label: str, value: bool) {
    print(label);
    println(value);
}

fn read_file(path: str) -> str {
    fp := fopen(path, "r\0");
    result :~ str = "";
    loop {
        byte := fread_byte(fp);
        if byte < 0 { break; }
        // Build string byte by byte (inefficient but works)
        ch := int_to_str(byte);
        result = result + ch;
    }
    fclose(fp);
    return result;
}

fn write_file(path: str, content: str) {
    fp := fopen(path, "w\0");
    fwrite_str(fp, content);
    fclose(fp);
}
