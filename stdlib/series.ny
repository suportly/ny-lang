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
        for item in self.data {
            total += item;
        }
        return total;
    }

    // Get the value at a specific index.
    // Note: No bounds checking for now, for performance.
    // This can be added later or a separate `get_safe` method could be created.
    pub fn get(self: &Self, index: usize) -> i64 {
        return self.data[index];
    }

    // Returns a new Series containing only the elements that satisfy the predicate.
    pub fn filter(self: &Self, predicate: fn(i64) -> bool) -> Self {
        let mut filtered_data = Vec::new();
        for item in self.data {
            if predicate(item) {
                filtered_data.push(item);
            }
        }
        return Series::new(self.name, filtered_data);
    }

    // Returns a new Series containing the results of applying the function to each element.
    pub fn map(self: &Self, func: fn(i64) -> i64) -> Self {
        let mut mapped_data = Vec::new();
        for item in self.data {
            mapped_data.push(func(item));
        }
        return Series::new(self.name, mapped_data);
    }

    // Returns the mean of the elements in the series.
    pub fn mean(self: &Self) -> i64 {
        if self.len() == 0 {
            return 0; // Or handle as an error, depending on desired behavior
        }
        return self.sum() / (self.len() as i64);
    }
}
