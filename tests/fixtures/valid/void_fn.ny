// Test: void functions without -> () annotation

fn greet(name: str) {
    print("Hello, ");
    println(name);
}

fn main() -> i32 {
    greet("World");
    return 42;
}
