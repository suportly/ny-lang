// Test: string .index_of() method

fn main() -> i32 {
    s := "hello world";

    // Find "world" at index 6
    idx := s.index_of("world");

    // Find "hello" at index 0
    idx2 := s.index_of("hello");

    // Find "xyz" — not found, returns -1
    idx3 := s.index_of("xyz");

    // Find "lo" at index 3
    idx4 := s.index_of("lo");

    // idx=6, idx2=0, idx3=-1, idx4=3
    // 6 + 0 + (-1) + 3 = 8
    // 8 + 34 = 42
    return idx + idx2 + idx3 + idx4 + 34;
}
