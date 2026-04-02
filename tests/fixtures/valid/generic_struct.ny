// Test: generic structs with monomorphization

struct Pair<A, B> {
    first: A,
    second: B,
}

fn get_first(p: Pair<i32, bool>) -> i32 {
    return p.first;
}

fn main() -> i32 {
    // Use the monomorphized struct name in init
    p := Pair_i32_bool { first: 42, second: true };
    return get_first(p);
}
