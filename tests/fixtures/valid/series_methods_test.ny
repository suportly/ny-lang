// tests/fixtures/valid/series_methods_test.ny
use stdlib::series::Series;
use std::vec::Vec;

fn is_even(x: i64) -> bool {
    return x % 2 == 0;
}

fn double_val(x: i64) -> i64 {
    return x * 2;
}

pub fn main() -> i32 {
    let mut data = Vec::new();
    data.push(10);
    data.push(20);
    data.push(30);
    data.push(40);
    data.push(50);
    
    let s = Series::new("test", data);
    
    // Test mean
    let mean_val = s.mean();
    if mean_val != 30 {
        return 1;
    }
    
    // Test filter
    let filtered_s = s.filter(is_even);
    if filtered_s.len() != 5 {
        return 2;
    }
    
    let mut data2 = Vec::new();
    data2.push(1);
    data2.push(2);
    data2.push(3);
    data2.push(4);
    data2.push(5);
    
    let s2 = Series::new("test2", data2);
    let filtered_s2 = s2.filter(is_even);
    if filtered_s2.len() != 2 {
        return 3;
    }
    if filtered_s2.get(0) != 2 {
        return 4;
    }
    if filtered_s2.get(1) != 4 {
        return 5;
    }
    
    // Test map
    let mapped_s2 = s2.map(double_val);
    if mapped_s2.len() != 5 {
        return 6;
    }
    if mapped_s2.get(0) != 2 {
        return 7;
    }
    if mapped_s2.get(4) != 10 {
        return 8;
    }
    
    return 42;
}
