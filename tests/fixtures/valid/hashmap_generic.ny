// Test: Generic HashMap<K,V> — multiple value types

fn main() -> i32 {
    // HashMap<str, i32>
    m1 :~ HashMap<str, i32> = hmap_new();
    m1.insert("a", 10);
    m1.insert("b", 20);
    v1 := m1.get("a");  // 10

    // HashMap<str, str>
    m2 :~ HashMap<str, str> = hmap_new();
    m2.insert("key", "hello");
    v2 := m2.get("key");  // "hello"

    // HashMap<str, f64>
    m3 :~ HashMap<str, f64> = hmap_new();
    m3.insert("pi", 3.14);
    v3 := m3.get("pi");  // 3.14

    // v1=10, v2.len()=5, v3 as i32 = 3
    // 10 + 5 + 3 + m1.len()=2 + m2.len()=1 + m3.len()=1 + 20 = 42
    return v1 + v2.len() as i32 + v3 as i32 + m1.len() as i32 + m2.len() as i32 + m3.len() as i32 + 20;
}
