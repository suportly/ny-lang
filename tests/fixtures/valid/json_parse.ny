// Test: JSON parser — objects, arrays, strings, numbers, booleans

fn main() -> i32 {
    // Parse object
    data := "{\"name\": \"Ny\", \"version\": 1, \"score\": 9.5, \"fast\": true}";
    obj := json_parse(data);
    defer json_free(obj);

    name := json_get_str(obj, "name");
    version := json_get_int(obj, "version");
    score := json_get_float(obj, "score");
    fast := json_get_bool(obj, "fast");

    // Parse array of objects
    arr_data := "[{\"v\": 10}, {\"v\": 20}, {\"v\": 12}]";
    arr := json_parse(arr_data);
    defer json_free(arr);

    total :~ i32 = 0;
    i :~ i32 = 0;
    while i < json_len(arr) {
        item := json_arr_get(arr, i);
        total += json_get_int(item, "v");
        i += 1;
    }

    // name.len()=2, version=1, fast=1, total=42
    // 2 + 1 + 1 + 42 - 4 = 42
    return name.len() as i32 + version + fast as i32 + total - 4;
}
