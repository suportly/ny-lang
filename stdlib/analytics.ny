// analytics.ny - A data-oriented standard library for high-performance analytics.

use std::io;
use std::string;
use std::vec::Vec;
use std::map::Map;

// A Series represents a single column of data.
// For now, we'll start with a series of f64.
// TODO: Make this generic to support different types.
pub struct Series {
    name: string,
    data: Vec<f64>,
}

pub fn (s: &Series) sum() -> f64 {
    let mut total: f64 = 0.0;
    for item in s.data {
        total += item;
    }
    return total;
}

pub fn (s: &Series) mean() -> f64 {
    if s.data.len() == 0 {
        return 0.0;
    }
    return s.sum() / (s.data.len() as f64);
}

// A DataFrame represents a collection of Series.
pub struct DataFrame {
    columns: Map<string, Series>,
}

// Reads a CSV file into a DataFrame.
// This is a simplified implementation and has many limitations:
// - Assumes all columns are f64.
// - No handling for missing values.
// - Basic parsing, no support for quoted fields, etc.
pub fn read_csv(path: string) -> DataFrame {
    let file = io::open(path);
    defer file.close();

    let mut lines = file.read_all_lines();

    if lines.len() < 2 { // Must have header and at least one data row
        return DataFrame{ columns: Map<string, Series>::new() };
    }

    let header_line = lines[0];
    let header = header_line.split(',');
    let mut df = DataFrame{ columns: Map<string, Series>::new() };
    let mut col_names = Vec<string>::new();

    for col_name_raw in header {
        let col_name = col_name_raw.strip();
        let series = Series {
            name: col_name,
            data: Vec<f64>::new(),
        };
        df.columns.set(col_name, series);
        col_names.push(col_name);
    }
    
    // remove header line
    lines.remove(0);

    for line in lines {
        let row = line.split(',');
        if row.len() != col_names.len() {
            // Skip malformed rows
            continue;
        }

        for i in 0..col_names.len() {
            let col_name = col_names[i];
            let mut series = df.columns.get(col_name);
            let value = row[i].strip().to_f64(); // Assumes conversion is successful
            series.data.push(value);
            df.columns.set(col_name, series);
        }
    }

    return df;
}
