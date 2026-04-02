// sort.ny — Sorting algorithms module

fn bubble_sort(arr: [10]i32) -> [10]i32 {
    result :~ [10]i32 = arr;
    n : i32 = 10;
    i :~ i32 = 0;
    while i < n - 1 {
        j :~ i32 = 0;
        while j < n - 1 - i {
            if result[j] > result[j + 1] {
                temp := result[j];
                result[j] = result[j + 1];
                result[j + 1] = temp;
            }
            j += 1;
        }
        i += 1;
    }
    return result;
}

fn is_sorted(arr: [10]i32) -> bool {
    i :~ i32 = 0;
    while i < 9 {
        if arr[i] > arr[i + 1] {
            return false;
        }
        i += 1;
    }
    return true;
}

fn selection_sort(arr: [10]i32) -> [10]i32 {
    result :~ [10]i32 = arr;
    n : i32 = 10;
    i :~ i32 = 0;
    while i < n - 1 {
        min_idx :~ i32 = i;
        j :~ i32 = i + 1;
        while j < n {
            if result[j] < result[min_idx] {
                min_idx = j;
            }
            j += 1;
        }
        if min_idx != i {
            temp := result[i];
            result[i] = result[min_idx];
            result[min_idx] = temp;
        }
        i += 1;
    }
    return result;
}

fn insertion_sort(arr: [10]i32) -> [10]i32 {
    result :~ [10]i32 = arr;
    n : i32 = 10;
    i :~ i32 = 1;
    while i < n {
        key := result[i];
        j :~ i32 = i - 1;
        while j >= 0 {
            if result[j] > key {
                result[j + 1] = result[j];
                j -= 1;
            } else {
                break;
            }
        }
        result[j + 1] = key;
        i += 1;
    }
    return result;
}
