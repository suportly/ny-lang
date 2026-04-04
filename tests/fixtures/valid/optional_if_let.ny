// if let val = optional — unwrap ?T with pattern matching

struct Point { x: i32, y: i32 }

fn find_point(make: bool) -> ?*Point {
    if make {
        return new Point { x: 21, y: 21 };
    }
    return nil;
}

fn main() -> i32 {
    p1 := find_point(true);
    p2 := find_point(false);

    // if let unwraps optional — val is *Point (non-optional)
    if let val = p1 {
        if val.x + val.y != 42 { return 1; }
    } else {
        return 2;
    }

    // p2 is nil — should go to else branch
    if let val = p2 {
        return 3;
    } else {
        // expected path
    }

    return 42;
}
