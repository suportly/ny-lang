extern {
    fn malloc(size: i32) -> *mut u8;
    fn free(ptr: *mut u8);
    fn memcpy(dest: *mut u8, src: *const u8, n: i32) -> *mut u8;
}

fn mock_gpu_launch_kernel_add(vec_a: *const u8, vec_b: *const u8, vec_out: *mut u8, size: i32) {
    count := size / 4;
    a_ptr := vec_a as *const i32;
    b_ptr := vec_b as *const i32;
    out_ptr := vec_out as *mut i32;
    
    i :~ i32 = 0;
    while i < count {
        a_val := *(a_ptr + i);
        b_val := *(b_ptr + i);
        *(out_ptr + i) = a_val + b_val;
        i += 1;
    }
}

fn main() -> i32 {
    count := 5;
    size := count * 4;
    
    host_a := malloc(size) as *mut i32;
    host_b := malloc(size) as *mut i32;
    host_out := malloc(size) as *mut i32;
    
    i :~ i32 = 0;
    while i < count {
        *(host_a + i) = i * 10;
        *(host_b + i) = i * 2 + 1;
        i += 1;
    }
    
    dev_a := malloc(size);
    dev_b := malloc(size);
    dev_out := malloc(size);
    
    memcpy(dev_a, host_a as *const u8, size);
    memcpy(dev_b, host_b as *const u8, size);
    
    mock_gpu_launch_kernel_add(dev_a as *const u8, dev_b as *const u8, dev_out, size);
    
    memcpy(host_out as *mut u8, dev_out as *const u8, size);
    
    j :~ i32 = 0;
    success :~ i32 = 1;
    while j < count {
        if *(host_out + j) != *(host_a + j) + *(host_b + j) {
            success = 0;
        }
        j += 1;
    }
    
    free(dev_a);
    free(dev_b);
    free(dev_out);
    free(host_a as *mut u8);
    free(host_b as *mut u8);
    free(host_out as *mut u8);
    
    if success == 1 {
        return 42;
    } else {
        return 1;
    }
}
