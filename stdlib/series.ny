// stdlib/series.ny
// A Series represents a single column of data.

use std::vec::Vec;
use std::mem;

// For now, a Series will hold i64 values.
// We can introduce generics or enums for different data types later.
pub struct Series {
    name: str,
    data: Vec<i64>,
}

impl Series {
    // Creates a new Series with a given name and data.
    pub fn new(name: str, data: Vec<i64>) -> Self {
        return Series { name: name, data: data };
    }

    // Returns the number of elements in the series.
    pub fn len(self: &Self) -> usize {
        return self.data.len();
    }

    // Returns the sum of all elements in the series.
    pub fn sum(self: &Self) -> i64 {
        let mut total: i64 = 0;
        let l = self.data.len();
        let mut i: usize = 0;
        while i < l {
            total += self.data[i];
            i += 1;
        }
        return total;
    }

    // Returns the mean (average) of all elements in the series.
    pub fn mean(self: &Self) -> i64 {
        if self.len() == 0 {
            return 0;
        }
        return self.sum() / (self.len() as i64);
    }

    // Get the value at a specific index.
    // Note: No bounds checking for now, for performance.
    // This can be added later or a separate `get_safe` method could be created.
    pub fn get(self: &Self, index: usize) -> i64 {
        return self.data[index];
    }
    
    // Filters the series based on a predicate function.
    // Returns a new Series with elements that satisfy the predicate.
    pub fn filter(self: &Self, predicate: fn(i64) -> bool) -> Self {
        let mut new_data = Vec::<i64>::new();
        let l = self.data.len();
        let mut i: usize = 0;
        while i < l {
            let val = self.data[i];
            if predicate(val) {
                new_data.push(val);
            }
            i += 1;
        }
        return Series::new(self.name, new_data);
    }

    // Maps the series using a mapping function.
    // Returns a new Series with the mapped elements.
    pub fn map(self: &Self, mapper: fn(i64) -> i64) -> Self {
        let mut new_data = Vec::<i64>::new();
        let l = self.data.len();
        let mut i: usize = 0;
        while i < l {
            new_data.push(mapper(self.data[i]));
            i += 1;
        }
        return Series::new(self.name, new_data);
    }
}
