// tests/fixtures/valid/series_methods_test.ny

use "std/series.ny"::Series;
use "std/vec.ny"::Vec;

fn is_even(x: i64) -> bool {
    return x % 2 == 0;
}

fn double_val(x: i64) -> i64 {
    return x * 2;
}

pub fn main(): i32 {
    let mut v = Vec::<i64>::new();
    v.push(10);
    v.push(15);
    v.push(20);
    v.push(25);
    v.push(30);

    let s = Series::new("my_series", v);

    // Test sum()
    if s.sum() != 100 {
        return 1;
    }

    // Test mean()
    if s.mean() != 20 {
        return 2;
    }

    // Test filter()
    let evens = s.filter(is_even);
    if evens.len() != 3 {
        return 3;
    }
    if evens.get(0) != 10 {
        return 4;
    }
    if evens.get(1) != 20 {
        return 5;
    }
    if evens.get(2) != 30 {
        return 6;
    }

    // Test map()
    let doubled = s.map(double_val);
    if doubled.len() != 5 {
        return 7;
    }
    if doubled.get(0) != 20 {
        return 8;
    }
    if doubled.get(1) != 30 {
        return 9;
    }
    if doubled.get(4) != 60 {
        return 10;
    }

    // All tests passed
    return 42;
}
