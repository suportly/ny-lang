// Test: multi-field enum variant payloads

enum Shape {
    Circle(i32),
    Rect(i32, i32),
}

fn area(s: Shape) -> i32 {
    return match s {
        Shape::Circle(r) => 3 * r * r,
        Shape::Rect(w, h) => w * h,
    };
}

fn main() -> i32 {
    c := Shape::Circle(3);
    r := Shape::Rect(4, 5);

    // 3*9 + 4*5 = 27 + 20 = 47
    return area(c) + area(r);
}
