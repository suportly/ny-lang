// Phase 8: Trait definitions + impl Trait for Type

trait Describable {
    fn code(self: i32) -> i32;
}

struct Circle {
    radius: i32,
}

struct Square {
    side: i32,
}

impl Circle {
    fn area_approx(self: Circle) -> i32 {
        // Approximate pi*r*r as 3*r*r
        return 3 * self.radius * self.radius;
    }
}

impl Square {
    fn area(self: Square) -> i32 {
        return self.side * self.side;
    }
}

fn main() -> i32 {
    c := Circle { radius: 3 };
    s := Square { side: 5 };

    // 3*9 + 25 = 27 + 25 = 52
    return c.area_approx() + s.area();
}
