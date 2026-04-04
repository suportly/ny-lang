// Dynamic dispatch with dyn Trait — plugin system pattern
//
// Define an interface, implement it for multiple types,
// and use dynamic dispatch to call methods polymorphically.
trait Plugin {
    fn execute(self: i32) -> i32;
}

struct Doubler {
    factor: i32,
}

impl Plugin for Doubler {
    fn execute(self: Doubler) -> i32 {
        return self.factor * 2;
    }
}

struct Adder {
    base: i32,
}

impl Plugin for Adder {
    fn execute(self: Adder) -> i32 {
        return self.base + 100;
    }
}
// Factory: returns dyn Plugin (interface value)

fn make_plugin(kind: i32, value: i32) -> dyn Plugin {
    if kind == 0 {
        return new Doubler { factor: value };
    }
    return new Adder { base: value };
}
// Polymorphic function — accepts any Plugin

fn run_plugin(p: dyn Plugin) -> i32 {
    return p.execute();
}

fn main() -> i32 {
    // Create plugins via factory
    d := make_plugin(0, 21); // Doubler { factor: 21 }
    a := make_plugin(1, 50); // Adder { base: 50 }
    // Dynamic dispatch — correct method called via vtable
    r1 := run_plugin(d); // 21 * 2 = 42
    r2 := run_plugin(a); // 50 + 100 = 150
    println("doubler:", r1); // 42
    println("adder:", r2); // 150
    return 0;
}
