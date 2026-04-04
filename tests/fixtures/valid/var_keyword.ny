// Phase 36: var keyword — readable mutable variable declaration

fn main() -> i32 {
    // var x = expr; is equivalent to x :~= expr;
    var total = 0;
    var i = 0;

    while i < 10 {
        total = total + i;
        i = i + 1;
    }

    if total != 45 { return 1; }

    // var with type annotation
    var name : i32 = 42;
    if name != 42 { return 2; }

    // Can mix with := (immutable)
    result := total - 3;
    if result != 42 { return 3; }

    return 42;
}
