// Phase 35: ?T optional types + ?? null coalescing

struct Point {
    x: i32,
    y: i32,
}

fn main() -> i32 {
    // ?*Point — optional pointer (can be nil)
    p1 : ?*Point = new Point { x: 10, y: 32 };
    p2 : ?*Point = nil;

    // Nil coalescing: ?? unwraps or provides default
    fallback := new Point { x: 0, y: 42 };

    // p2 is nil, so ?? returns fallback
    result := p2 ?? fallback;
    // result is *Point (unwrapped), safe to access fields
    if result.x != 0 { return 1; }
    if result.y != 42 { return 2; }

    // p1 is not nil, so ?? returns p1
    result2 := p1 ?? fallback;
    if result2.x != 10 { return 3; }

    // Direct nil comparison on optional
    if p1 == nil { return 4; }
    if p2 != nil { return 5; }

    return 42;
}
