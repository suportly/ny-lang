// GC roots across call frames: an object held by a caller must survive a
// collection triggered deeper in the call stack, and a pointer passed as a
// parameter must keep its object alive too.

struct Box {
    n: i32,
}

// Allocates enough garbage to cross the 1MB threshold while the caller's
// locals are live further up the stack.
fn churn(rounds: i32) -> i32 {
    i :~ i32 = 0;
    while i < rounds {
        new Box { n: i };
        i = i + 1;
    }
    return rounds;
}

// `b` arrives as a parameter: it is a root for this frame as well.
fn use_after_churn(b: *Box) -> i32 {
    churn(8000);
    return b.n;
}

fn main() -> i32 {
    outer := new Box { n: 21 };

    // Collection happens inside churn(), one frame below.
    churn(8000);
    if outer.n != 21 { return 1; }

    // Collection happens two frames below, with the pointer passed as an arg.
    if use_after_churn(outer) != 21 { return 2; }

    // Still intact after everything.
    gc_collect();
    if outer.n != 21 { return 3; }

    return 42;
}
