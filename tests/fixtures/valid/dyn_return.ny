// Phase 32: Functions returning dyn Trait

trait Animal {
    fn legs(self: i32) -> i32;
}

struct Dog { name: i32 }
impl Animal for Dog {
    fn legs(self: Dog) -> i32 { return 4; }
}

struct Spider { name: i32 }
impl Animal for Spider {
    fn legs(self: Spider) -> i32 { return 8; }
}

fn make_animal(kind: i32) -> dyn Animal {
    if kind == 0 {
        return new Dog { name: 1 };
    }
    return new Spider { name: 2 };
}

fn main() -> i32 {
    dog := make_animal(0);
    spider := make_animal(1);

    d := dog.legs();
    s := spider.legs();

    if d != 4 { return 1; }
    if s != 8 { return 2; }

    // 4 + 8 + 30 = 42
    return d + s + 30;
}
