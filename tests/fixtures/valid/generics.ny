// Test: generic functions with monomorphization

fn max<T>(a: T, b: T) -> T {
    if a > b {
        return a;
    }
    return b;
}

fn identity<T>(x: T) -> T {
    return x;
}

fn main() -> i32 {
    // max<i32>(10, 42) → monomorphized as max_i32
    a := max(10, 42);       // 42

    // identity<i32>(a) → monomorphized as identity_i32
    b := identity(a);       // 42

    return b;
}
