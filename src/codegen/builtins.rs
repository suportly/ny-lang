//! Builtin function registry for Ny Lang.
//!
//! Centralizes the return type and metadata of all builtin functions,
//! eliminating the duplicated if/else chains in typechecker and codegen.

use crate::common::NyType;

/// Return type for a builtin function, given knowledge of argument types.
pub fn builtin_return_type(name: &str, _arg_types: &[NyType]) -> Option<NyType> {
    match name {
        // I/O
        "print" | "println" => Some(NyType::Unit),

        // Memory
        "alloc" | "fopen" | "arena_new" | "arena_alloc" | "map_new" => {
            Some(NyType::Pointer(Box::new(NyType::U8)))
        }
        "free" | "arena_free" | "arena_reset" | "map_insert" | "map_remove" | "map_free"
        | "sleep_ms" | "exit" => {
            Some(NyType::Unit)
        }
        "sizeof" | "arena_bytes_used" | "map_len" | "vec_len" => Some(NyType::I64),

        // File I/O
        "fclose" | "fread_byte" | "fwrite_str" | "map_get" | "str_to_int" => Some(NyType::I32),
        "map_contains" => Some(NyType::Bool),

        // Generic HashMap
        "hmap_new" => Some(NyType::HashMap(Box::new(NyType::Str), Box::new(NyType::I32))),

        // String→String Map
        "smap_new" => Some(NyType::Pointer(Box::new(NyType::U8))),
        "smap_insert" => Some(NyType::Unit),
        "smap_get" => Some(NyType::Str),
        "smap_contains" => Some(NyType::Bool),
        "smap_len" => Some(NyType::I64),
        "smap_free" => Some(NyType::Unit),
        "map_key_at" => Some(NyType::Str),

        // Strings
        "read_line" | "int_to_str" | "float_to_str" | "read_file" => Some(NyType::Str),
        "str_to_float" => Some(NyType::F64),
        "write_file" | "remove_file" => Some(NyType::I32),

        // Vec
        "vec_new" => Some(NyType::Vec(Box::new(NyType::I32))),
        "vec_push" => Some(NyType::Unit),
        "vec_get" => Some(NyType::I32), // refined by method call handler

        // SIMD
        "simd_splat_f32x4" | "simd_load_f32x4" => Some(NyType::Simd {
            elem: Box::new(NyType::F32),
            lanes: 4,
        }),
        "simd_splat_f32x8" | "simd_load_f32x8" => Some(NyType::Simd {
            elem: Box::new(NyType::F32),
            lanes: 8,
        }),
        "simd_store_f32x4" | "simd_store_f32x8" => Some(NyType::Unit),
        "simd_reduce_add_f32" => Some(NyType::F32),
        "thread_spawn" => Some(NyType::I64),
        "thread_join" => Some(NyType::Unit),
        "to_str" => Some(NyType::Str),

        // Channels
        "channel_new" => Some(NyType::Pointer(Box::new(NyType::U8))),
        "channel_send" => Some(NyType::Unit),
        "channel_recv" => Some(NyType::I32),
        "channel_close" => Some(NyType::Unit),

        // Thread Pool
        "pool_new" => Some(NyType::Pointer(Box::new(NyType::U8))),
        "pool_submit" => Some(NyType::Unit),
        "pool_wait" => Some(NyType::Unit),
        "pool_free" => Some(NyType::Unit),

        // Parallel Iterators
        "par_map" => Some(NyType::Unit),
        "par_reduce" => Some(NyType::I32),

        // Math (f64)
        "sqrt" | "sin" | "cos" | "floor" | "ceil" | "fabs" | "log" | "exp" => {
            Some(NyType::F64)
        }
        "pow" => Some(NyType::F64),

        // Timing
        "clock_ms" => Some(NyType::I64),

        // JSON
        "json_parse" => Some(NyType::Pointer(Box::new(NyType::U8))),
        "json_type" => Some(NyType::I32),
        "json_get_int" => Some(NyType::I32),
        "json_get_float" => Some(NyType::F64),
        "json_get_str" => Some(NyType::Str),
        "json_get_bool" => Some(NyType::Bool),
        "json_len" => Some(NyType::I32),
        "json_arr_get" => Some(NyType::Pointer(Box::new(NyType::U8))),
        "json_free" => Some(NyType::Unit),

        // Tensor
        "tensor_zeros" | "tensor_ones" | "tensor_fill" | "tensor_rand" | "tensor_clone"
        | "tensor_add" | "tensor_sub" | "tensor_mul" | "tensor_scale"
        | "tensor_matmul" | "tensor_transpose" => {
            Some(NyType::Pointer(Box::new(NyType::U8)))
        }
        "tensor_get" | "tensor_sum" | "tensor_max" | "tensor_min"
        | "tensor_dot" | "tensor_norm" => Some(NyType::F64),
        "tensor_set" | "tensor_free" | "tensor_print" | "tensor_apply" => Some(NyType::Unit),
        "tensor_rows" | "tensor_cols" => Some(NyType::I64),

        // String split
        "str_split_count" => Some(NyType::I32),
        "str_split_get" => Some(NyType::Str),

        _ => None,
    }
}

/// Check if a name is a builtin function.
pub fn is_builtin(name: &str) -> bool {
    builtin_return_type(name, &[]).is_some()
}

/// All builtin function names (for resolver).
pub const BUILTIN_NAMES: &[&str] = &[
    "print",
    "println",
    "alloc",
    "free",
    "sizeof",
    "fopen",
    "fclose",
    "fwrite_str",
    "fread_byte",
    "exit",
    "sleep_ms",
    "read_line",
    "str_to_int",
    "int_to_str",
    "float_to_str",
    "str_to_float",
    "read_file",
    "write_file",
    "remove_file",
    "vec_new",
    "vec_push",
    "vec_len",
    "vec_get",
    "map_new",
    "map_insert",
    "map_get",
    "map_contains",
    "map_remove",
    "map_free",
    "map_key_at",
    "smap_new",
    "smap_insert",
    "smap_get",
    "smap_contains",
    "smap_len",
    "smap_free",
    "hmap_new",
    "tensor_zeros",
    "tensor_ones",
    "tensor_fill",
    "tensor_rand",
    "tensor_clone",
    "tensor_free",
    "tensor_rows",
    "tensor_cols",
    "tensor_get",
    "tensor_set",
    "tensor_add",
    "tensor_sub",
    "tensor_mul",
    "tensor_scale",
    "tensor_matmul",
    "tensor_transpose",
    "tensor_sum",
    "tensor_max",
    "tensor_min",
    "tensor_print",
    "tensor_dot",
    "tensor_norm",
    "tensor_apply",
    "map_len",
    "arena_new",
    "arena_alloc",
    "arena_free",
    "arena_reset",
    "arena_bytes_used",
    "simd_splat_f32x4",
    "simd_splat_f32x8",
    "simd_load_f32x4",
    "simd_load_f32x8",
    "simd_store_f32x4",
    "simd_store_f32x8",
    "simd_reduce_add_f32",
    "thread_spawn",
    "thread_join",
    "to_str",
    "channel_new",
    "channel_send",
    "channel_recv",
    "channel_close",
    "pool_new",
    "pool_submit",
    "pool_wait",
    "pool_free",
    "par_map",
    "par_reduce",
    "sqrt",
    "sin",
    "cos",
    "floor",
    "ceil",
    "fabs",
    "log",
    "exp",
    "pow",
    "clock_ms",
    "json_parse",
    "json_type",
    "json_get_int",
    "json_get_float",
    "json_get_str",
    "json_get_bool",
    "json_len",
    "json_arr_get",
    "json_free",
    "str_split_count",
    "str_split_get",
];
