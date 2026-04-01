struct Vec2 {
    x: i32,
    y: i32,
}

fn dot(self: Vec2, other: Vec2) -> i32 {
    return self.x * other.x + self.y * other.y;
}

fn main() -> i32 {
    a : Vec2 = Vec2 { x: 3, y: 4 };
    b : Vec2 = Vec2 { x: 1, y: 2 };
    return a.dot(b);
}
