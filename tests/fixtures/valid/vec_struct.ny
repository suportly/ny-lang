// Test: Vec<struct> — vector of user-defined struct types

struct Point { x: i32, y: i32 }

fn main() -> i32 {
    v :~ Vec<Point> = vec_new();
    v.push(Point { x: 10, y: 20 });
    v.push(Point { x: 5, y: 7 });
    v.push(Point { x: 30, y: 12 });

    // Access elements
    p0 := v.get(0);
    p2 := v.get(2);

    // p0.x=10, p2.y=12, v.len()=3
    // 10 + 12 + 3 + 17 = 42
    return p0.x + p2.y + v.len() as i32 + 17;
}
