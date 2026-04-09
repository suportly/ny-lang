// tests/fixtures/valid/series_test.ny

// We need to specify the path to the standard library modules
use "std/series.ny"::Series;
use "std/vec.ny"::Vec;

pub fn main(): i32 {
    let mut v = Vec<i64>::new();
    v.push(10);
    v.push(20);
    v.push(30);
    v.push(40);
    v.push(50);

    let s = Series::new("my_series", v);

    // Test len()
    if s.len() != 5 {
        return 1;
    }

    // Test sum()
    if s.sum() != 150 {
        return 2;
    }

    // Test get()
    if s.get(0) != 10 {
        return 3;
    }
    if s.get(2) != 30 {
        return 4;
    }
    if s.get(4) != 50 {
        return 5;
    }

    // All tests passed
    return 42;
}
