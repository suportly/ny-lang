// Test: operator overloading via impl methods

struct Vec2 { x: i32, y: i32 }

impl Vec2 {
    fn add(self: Vec2, other: Vec2) -> Vec2 {
        return Vec2 { x: self.x + other.x, y: self.y + other.y };
    }

    fn sub(self: Vec2, other: Vec2) -> Vec2 {
        return Vec2 { x: self.x - other.x, y: self.y - other.y };
    }

    fn mul(self: Vec2, other: Vec2) -> Vec2 {
        return Vec2 { x: self.x * other.x, y: self.y * other.y };
    }
}

fn main() -> i32 {
    a := Vec2 { x: 10, y: 20 };
    b := Vec2 { x: 5, y: 8 };

    c := a + b;  // Vec2 { x: 15, y: 28 }
    d := a - b;  // Vec2 { x: 5, y: 12 }
    e := a * b;  // Vec2 { x: 50, y: 160 }

    // c.x=15, d.y=12, e.x=50
    // 15 + 12 + 50 - 35 = 42
    return c.x + d.y + e.x - 35;
}
