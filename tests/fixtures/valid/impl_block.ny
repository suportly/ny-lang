// Phase 7: impl blocks

struct Point {
    x: i32,
    y: i32,
}

impl Point {
    fn magnitude_sq(self: Point) -> i32 {
        return self.x * self.x + self.y * self.y;
    }

    fn add(self: Point, other: Point) -> Point {
        return Point { x: self.x + other.x, y: self.y + other.y };
    }
}

fn main() -> i32 {
    p1 := Point { x: 3, y: 4 };
    p2 := Point { x: 1, y: 2 };
    p3 := p1.add(p2);

    // magnitude_sq of (4,6) = 16+36 = 52
    return p3.magnitude_sq();
}
