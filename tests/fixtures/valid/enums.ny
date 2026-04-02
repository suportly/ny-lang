enum Color {
    Red,
    Green,
    Blue,
}

fn color_to_int(c: Color) -> i32 {
    return match c {
        Color::Red => 1,
        Color::Green => 2,
        Color::Blue => 3,
    };
}

fn main() -> i32 {
    c := Color::Green;
    return color_to_int(c);
}
