// Error: non-exhaustive match on enum
enum Color { Red, Green, Blue }

fn main() -> i32 {
    c : Color = Color::Red;
    x := match c {
        Color::Red => 1,
        Color::Green => 2,
    };
    return x;
}
