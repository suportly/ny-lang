// interface keyword as alias for trait (Go-style naming)

interface Greeter {
    fn greet(self: i32) -> i32;
}

struct Hello { value: i32 }

impl Greeter for Hello {
    fn greet(self: Hello) -> i32 {
        return self.value;
    }
}

fn main() -> i32 {
    h := new Hello { value: 42 };
    g : dyn Greeter = h;
    return g.greet();
}
