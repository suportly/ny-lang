// Go-style programming in Ny: GC + interfaces + goroutines + channels + nil
// Demonstrates the full "Go with algebraic types" pattern

// --- Trait (interface) ---

trait Processor {
    fn process(self: i32) -> i32;
}

// --- Concrete types ---

struct Doubler { factor: i32 }

impl Processor for Doubler {
    fn process(self: Doubler) -> i32 {
        return self.factor * 2;
    }
}

struct Adder { base: i32 }

impl Processor for Adder {
    fn process(self: Adder) -> i32 {
        return self.base + 10;
    }
}

// --- Worker function (runs in goroutine) ---

fn worker(ch: *u8, value: i32) {
    channel_send(ch, value);
}

// --- Polymorphic function ---

fn run_processor(p: dyn Processor) -> i32 {
    return p.process();
}

fn main() -> i32 {
    // 1. GC-managed allocations (no free needed)
    d := new Doubler { factor: 7 };
    a := new Adder { base: 4 };

    // 2. Dynamic dispatch via dyn Trait
    p1 : dyn Processor = d;
    p2 : dyn Processor = a;

    r1 := run_processor(p1);  // 7 * 2 = 14
    r2 := run_processor(p2);  // 4 + 10 = 14

    if r1 != 14 { return 1; }
    if r2 != 14 { return 2; }

    // 3. Goroutines + channels
    ch := channel_new(16);

    go worker(ch, r1);
    go worker(ch, r2);
    go worker(ch, 14);

    v1 := channel_recv(ch);
    v2 := channel_recv(ch);
    v3 := channel_recv(ch);

    total := v1 + v2 + v3;
    if total != 42 { return 3; }

    channel_close(ch);

    // 4. nil checking
    ptr : *u8 = nil;
    if ptr != nil { return 4; }

    safe := new Doubler { factor: 21 };
    if safe == nil { return 5; }

    // 5. Typed channels
    tch : chan<i32> = chan_new(4);
    tch.send(42);
    result := tch.recv();
    tch.close();

    if result != 42 { return 6; }

    // 6. Multiple return values (already works)
    (quot, ok) := divide(84, 2);
    if !ok { return 7; }
    if quot != 42 { return 8; }

    return 42;
}

fn divide(a: i32, b: i32) -> (i32, bool) {
    if b == 0 { return (0, false); }
    return (a / b, true);
}
