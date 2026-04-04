// Negative test: accessing field on ?*T should be compile error

struct Point { x: i32, y: i32 }

fn main() -> i32 {
    p : ?*Point = nil;
    return p.x;  // ERROR: cannot access field on optional type
}
