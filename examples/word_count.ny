// Word Count — Demonstrates: HashMap, File I/O, loops, string ops, extern FFI
//
// Reads a text file byte-by-byte, counts total lines, words, and characters.
// Also tracks frequency of each word length using HashMap.
//
// Usage: word_count (creates a sample file, then counts it)

extern {
    fn putchar(c: i32) -> i32;
}

fn main() -> i32 {
    // Create a sample text file to count
    sample := "the quick brown fox jumps over the lazy dog\nthe fox is quick and the dog is lazy\nhello world of ny lang programming\n\0";

    fp := fopen("/tmp/ny_wordcount.txt\0", "w\0");
    fwrite_str(fp, sample);
    fclose(fp);

    // Now read and count
    fp2 := fopen("/tmp/ny_wordcount.txt\0", "r\0");

    lines :~ i32 = 0;
    words :~ i32 = 0;
    chars :~ i32 = 0;
    in_word :~ i32 = 0;
    word_len :~ i32 = 0;

    // HashMap to track word-length frequencies
    len_freq := map_new();

    b :~ i32 = fread_byte(fp2);
    while b >= 0 {
        chars += 1;

        is_space :~ i32 = 0;
        if b == 32 { is_space = 1; }      // space
        if b == 10 { is_space = 1; }      // newline
        if b == 9 { is_space = 1; }       // tab
        if b == 13 { is_space = 1; }      // carriage return

        if b == 10 {
            lines += 1;
        }

        if is_space == 1 {
            if in_word == 1 {
                words += 1;
                // Record word length frequency
                key := int_to_str(word_len);
                prev := map_get(len_freq, key);
                map_insert(len_freq, key, prev + 1);
                word_len = 0;
            }
            in_word = 0;
        } else {
            in_word = 1;
            word_len += 1;
        }

        b = fread_byte(fp2);
    }

    // Handle last word if file doesn't end with whitespace
    if in_word == 1 {
        words += 1;
        key := int_to_str(word_len);
        prev := map_get(len_freq, key);
        map_insert(len_freq, key, prev + 1);
    }

    fclose(fp2);

    // Report results
    println("=== Word Count Results ===");
    println("");

    print("  Lines: ");
    println(lines);

    print("  Words: ");
    println(words);

    print("  Chars: ");
    println(chars);

    println("");
    println("Word length distribution:");

    // Check lengths 1 through 11
    i :~ i32 = 1;
    while i <= 11 {
        key := int_to_str(i);
        if map_contains(len_freq, key) {
            count := map_get(len_freq, key);
            print("  len ");
            print(i);
            print(": ");
            // Print a simple bar chart
            j :~ i32 = 0;
            while j < count {
                putchar(35);  // '#'
                j += 1;
            }
            putchar(32);  // space
            putchar(40);  // '('
            print(count);
            println(")");
        }
        i += 1;
    }

    println("");
    println("=== Done ===");

    return 0;
}
