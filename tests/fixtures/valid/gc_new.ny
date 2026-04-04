// Phase 26: `new` keyword — GC-managed struct allocation
// new Type { fields } allocates on the GC heap and returns *Type

struct Point {
    x: i32,
    y: i32,
}

impl Point {
    fn sum(self: Point) -> i32 {
        return self.x + self.y;
    }
}

struct Rect {
    w: i32,
    h: i32,
}

impl Rect {
    fn area(self: Rect) -> i32 {
        return self.w * self.h;
    }
}

fn main() -> i32 {
    // new allocates on GC heap, returns *Point
    p := new Point { x: 10, y: 32 };

    // Auto-deref: field access through pointer works
    if p.x != 10 { return 1; }
    if p.y != 32 { return 2; }

    // Method call through pointer (auto-deref)
    if p.sum() != 42 { return 3; }

    // Multiple GC-managed allocations
    r := new Rect { w: 6, h: 7 };
    if r.area() != 42 { return 4; }

    // Allocate in a loop — GC handles cleanup
    i :~ i32 = 0;
    while i < 50 {
        new Point { x: i, y: i * 2 };
        i = i + 1;
    }

    // Force collection to prove nothing crashes
    gc_collect();

    // Verify the original pointers still work after GC
    // (they are stack roots, so they won't be collected)
    if p.sum() != 42 { return 5; }
    if r.area() != 42 { return 6; }

    return 42;
}
