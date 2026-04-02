// Error: impl Describable for Point is missing method 'describe'
trait Describable {
    fn describe(self: i32) -> i32;
}

struct Point {
    x: i32,
    y: i32,
}

impl Describable for Point {
    // Missing 'describe' method!
}

fn main() -> i32 {
    return 0;
}
