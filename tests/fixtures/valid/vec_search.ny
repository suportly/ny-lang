// Test: Vec.contains() and Vec.index_of()

fn main() -> i32 {
    v :~ Vec<i32> = vec_new();
    v.push(10);
    v.push(20);
    v.push(30);
    v.push(42);
    v.push(50);

    // contains
    has_42 := v.contains(42);    // true
    has_99 := v.contains(99);    // false

    // index_of
    idx := v.index_of(42);     // 3
    idx2 := v.index_of(10);    // 0
    idx3 := v.index_of(99);    // -1

    result :~ i32 = 0;
    if has_42 { result += 10; }
    if !has_99 { result += 10; }

    // idx=3, idx2=0, idx3=-1
    // 3 + 0 + (-1) = 2
    result += idx + idx2 + idx3;

    // 10 + 10 + 2 = 22, need +20
    result += 20;

    return result;
}
