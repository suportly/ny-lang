// Phase 27: dyn Trait — dynamic dispatch via vtables
// Tests: trait objects, virtual method calls, polymorphism

trait Shape {
    fn area(self: i32) -> i32;
}

struct Circle {
    radius: i32,
}

impl Shape for Circle {
    fn area(self: Circle) -> i32 {
        return self.radius * self.radius * 3;
    }
}

struct Rect {
    w: i32,
    h: i32,
}

impl Shape for Rect {
    fn area(self: Rect) -> i32 {
        return self.w * self.h;
    }
}

fn compute_area(s: dyn Shape) -> i32 {
    return s.area();
}

fn main() -> i32 {
    // Create concrete instances on GC heap
    c := new Circle { radius: 3 };
    r := new Rect { w: 6, h: 7 };

    // Coerce to dyn Shape
    s1 : dyn Shape = c;
    s2 : dyn Shape = r;

    // Dynamic dispatch — calls Circle.area via vtable
    a1 := s1.area();
    if a1 != 27 { return 1; }

    // Dynamic dispatch — calls Rect.area via vtable
    a2 := s2.area();
    if a2 != 42 { return 2; }

    // Pass dyn trait to function
    a3 := compute_area(s1);
    if a3 != 27 { return 3; }

    a4 := compute_area(s2);
    if a4 != 42 { return 4; }

    return 42;
}
