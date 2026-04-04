// Phase 30: nil literal — null pointer value and comparison
// Tests: nil literal, pointer == nil, pointer != nil

struct Node {
    value: i32,
    next: *u8,
}

fn main() -> i32 {
    // nil is a null pointer
    p : *u8 = nil;

    // Compare pointer with nil
    if p != nil { return 1; }

    // Allocate something — not nil
    n := new Node { value: 42, next: nil };
    if n == nil { return 2; }

    // Access through non-nil pointer
    if n.value != 42 { return 3; }

    // Check that the next field is nil
    if n.next != nil { return 4; }

    return 42;
}
