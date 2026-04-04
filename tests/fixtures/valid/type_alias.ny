// Phase 33: type keyword — type aliases

type Meters = f64;
type Count = i32;

struct Vec2 { x: f64, y: f64 }

type Point2D = Vec2;

fn distance(a: Meters, b: Meters) -> Meters {
    return a + b;
}

fn count_items(n: Count) -> Count {
    return n * 2;
}

fn main() -> i32 {
    d : Meters = 3.14;
    e : Meters = 2.86;
    total := distance(d, e);

    n : Count = 21;
    doubled := count_items(n);

    if doubled != 42 { return 1; }

    // Type alias for struct
    p : Point2D = Vec2 { x: 1.0, y: 2.0 };

    return 42;
}
