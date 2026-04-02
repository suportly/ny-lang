// Test: generic function with trait bounds syntax (parsing only, bounds not enforced yet)

trait Printable {
    fn describe(self: i32) -> i32;
}

impl Printable for i32 {
    fn describe(self: i32) -> i32 {
        return self;
    }
}

fn identity<T: Printable>(x: T) -> T {
    return x;
}

fn main() -> i32 {
    result := identity(42);
    return result;
}
