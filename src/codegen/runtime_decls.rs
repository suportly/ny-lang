//! C runtime and libc function declarations for Ny codegen.
//!
//! All `get_or_declare_*` helpers live here. They lazily declare
//! external C functions (or globals) in the LLVM module so that the
//! rest of codegen can simply call them.

use inkwell::values::{FunctionValue, GlobalValue};
use inkwell::AddressSpace;

use super::CodeGen;

impl<'ctx> CodeGen<'ctx> {
    pub(super) fn get_or_declare_c_fn(
        &self,
        name: &str,
        fn_type: inkwell::types::FunctionType<'ctx>,
    ) -> FunctionValue<'ctx> {
        if let Some(f) = self.module.get_function(name) {
            return f;
        }
        self.module.add_function(name, fn_type, None)
    }

    pub(super) fn get_or_declare_pthread_create(&self) -> FunctionValue<'ctx> {
        if let Some(f) = self.module.get_function("pthread_create") {
            return f;
        }
        let ptr_ty = self.context.ptr_type(AddressSpace::default());
        let i32_ty = self.context.i32_type();
        let fn_ty = i32_ty.fn_type(
            &[ptr_ty.into(), ptr_ty.into(), ptr_ty.into(), ptr_ty.into()],
            false,
        );
        self.module.add_function("pthread_create", fn_ty, None)
    }

    pub(super) fn get_or_declare_pthread_join(&self) -> FunctionValue<'ctx> {
        if let Some(f) = self.module.get_function("pthread_join") {
            return f;
        }
        let i64_ty = self.context.i64_type();
        let ptr_ty = self.context.ptr_type(AddressSpace::default());
        let fn_ty = self
            .context
            .i32_type()
            .fn_type(&[i64_ty.into(), ptr_ty.into()], false);
        self.module.add_function("pthread_join", fn_ty, None)
    }

    pub(super) fn get_or_declare_ny_arena_new(&self) -> FunctionValue<'ctx> {
        if let Some(f) = self.module.get_function("ny_arena_new") {
            return f;
        }
        let ptr_ty = self.context.ptr_type(AddressSpace::default());
        let i64_ty = self.context.i64_type();
        self.module.add_function(
            "ny_arena_new",
            ptr_ty.fn_type(&[i64_ty.into()], false),
            None,
        )
    }

    pub(super) fn get_or_declare_ny_arena_alloc(&self) -> FunctionValue<'ctx> {
        if let Some(f) = self.module.get_function("ny_arena_alloc") {
            return f;
        }
        let ptr_ty = self.context.ptr_type(AddressSpace::default());
        let i64_ty = self.context.i64_type();
        self.module.add_function(
            "ny_arena_alloc",
            ptr_ty.fn_type(&[ptr_ty.into(), i64_ty.into()], false),
            None,
        )
    }

    pub(super) fn get_or_declare_ny_arena_free(&self) -> FunctionValue<'ctx> {
        if let Some(f) = self.module.get_function("ny_arena_free") {
            return f;
        }
        let ptr_ty = self.context.ptr_type(AddressSpace::default());
        self.module.add_function(
            "ny_arena_free",
            self.context.void_type().fn_type(&[ptr_ty.into()], false),
            None,
        )
    }

    pub(super) fn get_or_declare_ny_arena_reset(&self) -> FunctionValue<'ctx> {
        if let Some(f) = self.module.get_function("ny_arena_reset") {
            return f;
        }
        let ptr_ty = self.context.ptr_type(AddressSpace::default());
        self.module.add_function(
            "ny_arena_reset",
            self.context.void_type().fn_type(&[ptr_ty.into()], false),
            None,
        )
    }

    pub(super) fn get_or_declare_ny_arena_bytes_used(&self) -> FunctionValue<'ctx> {
        if let Some(f) = self.module.get_function("ny_arena_bytes_used") {
            return f;
        }
        let ptr_ty = self.context.ptr_type(AddressSpace::default());
        let i64_ty = self.context.i64_type();
        self.module.add_function(
            "ny_arena_bytes_used",
            i64_ty.fn_type(&[ptr_ty.into()], false),
            None,
        )
    }

    pub(super) fn get_or_declare_ny_map_new(&self) -> FunctionValue<'ctx> {
        if let Some(f) = self.module.get_function("ny_map_new") {
            return f;
        }
        let ptr_ty = self.context.ptr_type(AddressSpace::default());
        let fn_ty = ptr_ty.fn_type(&[], false);
        self.module.add_function("ny_map_new", fn_ty, None)
    }

    pub(super) fn get_or_declare_ny_map_insert(&self) -> FunctionValue<'ctx> {
        if let Some(f) = self.module.get_function("ny_map_insert") {
            return f;
        }
        let ptr_ty = self.context.ptr_type(AddressSpace::default());
        let i64_ty = self.context.i64_type();
        let void_ty = self.context.void_type();
        let fn_ty = void_ty.fn_type(
            &[ptr_ty.into(), ptr_ty.into(), i64_ty.into(), i64_ty.into()],
            false,
        );
        self.module.add_function("ny_map_insert", fn_ty, None)
    }

    pub(super) fn get_or_declare_ny_map_get(&self) -> FunctionValue<'ctx> {
        if let Some(f) = self.module.get_function("ny_map_get") {
            return f;
        }
        let ptr_ty = self.context.ptr_type(AddressSpace::default());
        let i64_ty = self.context.i64_type();
        let fn_ty = i64_ty.fn_type(&[ptr_ty.into(), ptr_ty.into(), i64_ty.into()], false);
        self.module.add_function("ny_map_get", fn_ty, None)
    }

    pub(super) fn get_or_declare_ny_map_contains(&self) -> FunctionValue<'ctx> {
        if let Some(f) = self.module.get_function("ny_map_contains") {
            return f;
        }
        let ptr_ty = self.context.ptr_type(AddressSpace::default());
        let i64_ty = self.context.i64_type();
        let i32_ty = self.context.i32_type();
        let fn_ty = i32_ty.fn_type(&[ptr_ty.into(), ptr_ty.into(), i64_ty.into()], false);
        self.module.add_function("ny_map_contains", fn_ty, None)
    }

    pub(super) fn get_or_declare_ny_map_len(&self) -> FunctionValue<'ctx> {
        if let Some(f) = self.module.get_function("ny_map_len") {
            return f;
        }
        let ptr_ty = self.context.ptr_type(AddressSpace::default());
        let i64_ty = self.context.i64_type();
        let fn_ty = i64_ty.fn_type(&[ptr_ty.into()], false);
        self.module.add_function("ny_map_len", fn_ty, None)
    }

    pub(super) fn get_or_declare_ny_map_remove(&self) -> FunctionValue<'ctx> {
        if let Some(f) = self.module.get_function("ny_map_remove") {
            return f;
        }
        let ptr_ty = self.context.ptr_type(AddressSpace::default());
        let i64_ty = self.context.i64_type();
        let fn_ty = self
            .context
            .void_type()
            .fn_type(&[ptr_ty.into(), ptr_ty.into(), i64_ty.into()], false);
        self.module.add_function("ny_map_remove", fn_ty, None)
    }

    pub(super) fn get_or_declare_ny_map_key_at(&self) -> FunctionValue<'ctx> {
        if let Some(f) = self.module.get_function("ny_map_key_at") {
            return f;
        }
        let ptr_ty = self.context.ptr_type(AddressSpace::default());
        let i64_ty = self.context.i64_type();
        // const char *ny_map_key_at(NyHashMap *m, i64 index, i64 *out_len)
        let fn_ty = ptr_ty.fn_type(&[ptr_ty.into(), i64_ty.into(), ptr_ty.into()], false);
        self.module.add_function("ny_map_key_at", fn_ty, None)
    }

    // String→String Map
    pub(super) fn get_or_declare_ny_smap_new(&self) -> FunctionValue<'ctx> {
        if let Some(f) = self.module.get_function("ny_smap_new") { return f; }
        let ptr_ty = self.context.ptr_type(AddressSpace::default());
        self.module.add_function("ny_smap_new", ptr_ty.fn_type(&[], false), None)
    }
    pub(super) fn get_or_declare_ny_smap_insert(&self) -> FunctionValue<'ctx> {
        if let Some(f) = self.module.get_function("ny_smap_insert") { return f; }
        let ptr_ty = self.context.ptr_type(AddressSpace::default());
        let i64_ty = self.context.i64_type();
        let fn_ty = self.context.void_type().fn_type(
            &[ptr_ty.into(), ptr_ty.into(), i64_ty.into(), ptr_ty.into(), i64_ty.into()], false);
        self.module.add_function("ny_smap_insert", fn_ty, None)
    }
    pub(super) fn get_or_declare_ny_smap_get(&self) -> FunctionValue<'ctx> {
        if let Some(f) = self.module.get_function("ny_smap_get") { return f; }
        let ptr_ty = self.context.ptr_type(AddressSpace::default());
        let i64_ty = self.context.i64_type();
        let fn_ty = ptr_ty.fn_type(&[ptr_ty.into(), ptr_ty.into(), i64_ty.into(), ptr_ty.into()], false);
        self.module.add_function("ny_smap_get", fn_ty, None)
    }
    pub(super) fn get_or_declare_ny_smap_contains(&self) -> FunctionValue<'ctx> {
        if let Some(f) = self.module.get_function("ny_smap_contains") { return f; }
        let ptr_ty = self.context.ptr_type(AddressSpace::default());
        let i64_ty = self.context.i64_type();
        let i32_ty = self.context.i32_type();
        let fn_ty = i32_ty.fn_type(&[ptr_ty.into(), ptr_ty.into(), i64_ty.into()], false);
        self.module.add_function("ny_smap_contains", fn_ty, None)
    }
    pub(super) fn get_or_declare_ny_smap_len(&self) -> FunctionValue<'ctx> {
        if let Some(f) = self.module.get_function("ny_smap_len") { return f; }
        let ptr_ty = self.context.ptr_type(AddressSpace::default());
        let i64_ty = self.context.i64_type();
        self.module.add_function("ny_smap_len", i64_ty.fn_type(&[ptr_ty.into()], false), None)
    }
    // Generic HashMap<K,V>
    pub(super) fn get_or_declare_ny_hmap_new(&self) -> FunctionValue<'ctx> {
        if let Some(f) = self.module.get_function("ny_hmap_new") { return f; }
        let ptr_ty = self.context.ptr_type(AddressSpace::default());
        let i64_ty = self.context.i64_type();
        self.module.add_function("ny_hmap_new", ptr_ty.fn_type(&[i64_ty.into()], false), None)
    }
    pub(super) fn get_or_declare_ny_hmap_insert(&self) -> FunctionValue<'ctx> {
        if let Some(f) = self.module.get_function("ny_hmap_insert") { return f; }
        let ptr_ty = self.context.ptr_type(AddressSpace::default());
        let i64_ty = self.context.i64_type();
        let fn_ty = self.context.void_type().fn_type(
            &[ptr_ty.into(), ptr_ty.into(), i64_ty.into(), ptr_ty.into()], false);
        self.module.add_function("ny_hmap_insert", fn_ty, None)
    }
    pub(super) fn get_or_declare_ny_hmap_get(&self) -> FunctionValue<'ctx> {
        if let Some(f) = self.module.get_function("ny_hmap_get") { return f; }
        let ptr_ty = self.context.ptr_type(AddressSpace::default());
        let i64_ty = self.context.i64_type();
        let i32_ty = self.context.i32_type();
        let fn_ty = i32_ty.fn_type(&[ptr_ty.into(), ptr_ty.into(), i64_ty.into(), ptr_ty.into()], false);
        self.module.add_function("ny_hmap_get", fn_ty, None)
    }
    pub(super) fn get_or_declare_ny_hmap_contains(&self) -> FunctionValue<'ctx> {
        if let Some(f) = self.module.get_function("ny_hmap_contains") { return f; }
        let ptr_ty = self.context.ptr_type(AddressSpace::default());
        let i64_ty = self.context.i64_type();
        let i32_ty = self.context.i32_type();
        let fn_ty = i32_ty.fn_type(&[ptr_ty.into(), ptr_ty.into(), i64_ty.into()], false);
        self.module.add_function("ny_hmap_contains", fn_ty, None)
    }
    pub(super) fn get_or_declare_ny_hmap_len(&self) -> FunctionValue<'ctx> {
        if let Some(f) = self.module.get_function("ny_hmap_len") { return f; }
        let ptr_ty = self.context.ptr_type(AddressSpace::default());
        let i64_ty = self.context.i64_type();
        self.module.add_function("ny_hmap_len", i64_ty.fn_type(&[ptr_ty.into()], false), None)
    }
    pub(super) fn get_or_declare_ny_hmap_remove(&self) -> FunctionValue<'ctx> {
        if let Some(f) = self.module.get_function("ny_hmap_remove") { return f; }
        let ptr_ty = self.context.ptr_type(AddressSpace::default());
        let i64_ty = self.context.i64_type();
        let fn_ty = self.context.void_type().fn_type(&[ptr_ty.into(), ptr_ty.into(), i64_ty.into()], false);
        self.module.add_function("ny_hmap_remove", fn_ty, None)
    }
    pub(super) fn get_or_declare_ny_hmap_free(&self) -> FunctionValue<'ctx> {
        if let Some(f) = self.module.get_function("ny_hmap_free") { return f; }
        let ptr_ty = self.context.ptr_type(AddressSpace::default());
        self.module.add_function("ny_hmap_free", self.context.void_type().fn_type(&[ptr_ty.into()], false), None)
    }

    pub(super) fn get_or_declare_ny_smap_free(&self) -> FunctionValue<'ctx> {
        if let Some(f) = self.module.get_function("ny_smap_free") { return f; }
        let ptr_ty = self.context.ptr_type(AddressSpace::default());
        self.module.add_function("ny_smap_free", self.context.void_type().fn_type(&[ptr_ty.into()], false), None)
    }

    pub(super) fn get_or_declare_ny_map_free(&self) -> FunctionValue<'ctx> {
        if let Some(f) = self.module.get_function("ny_map_free") {
            return f;
        }
        let ptr_ty = self.context.ptr_type(AddressSpace::default());
        let fn_ty = self.context.void_type().fn_type(&[ptr_ty.into()], false);
        self.module.add_function("ny_map_free", fn_ty, None)
    }

    pub(super) fn get_or_declare_fflush(&self) -> FunctionValue<'ctx> {
        if let Some(f) = self.module.get_function("fflush") {
            return f;
        }
        let ptr_ty = self.context.ptr_type(AddressSpace::default());
        let i32_ty = self.context.i32_type();
        let fflush_ty = i32_ty.fn_type(&[ptr_ty.into()], false);
        self.module.add_function("fflush", fflush_ty, None)
    }

    // ------------------------------------------------------------------
    // Libc function declarations (lazy, idempotent)
    // ------------------------------------------------------------------

    pub(super) fn get_or_declare_printf(&self) -> FunctionValue<'ctx> {
        if let Some(f) = self.module.get_function("printf") {
            return f;
        }
        let ptr_ty = self.context.ptr_type(AddressSpace::default());
        let printf_ty = self.context.i32_type().fn_type(&[ptr_ty.into()], true); // variadic
        self.module.add_function("printf", printf_ty, None)
    }

    pub(super) fn get_or_declare_write(&self) -> FunctionValue<'ctx> {
        if let Some(f) = self.module.get_function("write") {
            return f;
        }
        let i32_ty = self.context.i32_type();
        let i64_ty = self.context.i64_type();
        let ptr_ty = self.context.ptr_type(AddressSpace::default());
        // ssize_t write(int fd, const void *buf, size_t count)
        let write_ty = i64_ty.fn_type(&[i32_ty.into(), ptr_ty.into(), i64_ty.into()], false);
        self.module.add_function("write", write_ty, None)
    }

    pub(super) fn get_or_declare_malloc(&self) -> FunctionValue<'ctx> {
        if let Some(f) = self.module.get_function("malloc") {
            return f;
        }
        let ptr_ty = self.context.ptr_type(AddressSpace::default());
        let i64_ty = self.context.i64_type();
        let malloc_ty = ptr_ty.fn_type(&[i64_ty.into()], false);
        self.module.add_function("malloc", malloc_ty, None)
    }

    pub(super) fn get_or_declare_free(&self) -> FunctionValue<'ctx> {
        if let Some(f) = self.module.get_function("free") {
            return f;
        }
        let ptr_ty = self.context.ptr_type(AddressSpace::default());
        let free_ty = self.context.void_type().fn_type(&[ptr_ty.into()], false);
        self.module.add_function("free", free_ty, None)
    }

    pub(super) fn get_or_declare_realloc(&self) -> FunctionValue<'ctx> {
        if let Some(f) = self.module.get_function("realloc") {
            return f;
        }
        let ptr_ty = self.context.ptr_type(AddressSpace::default());
        let i64_ty = self.context.i64_type();
        let realloc_ty = ptr_ty.fn_type(&[ptr_ty.into(), i64_ty.into()], false);
        self.module.add_function("realloc", realloc_ty, None)
    }

    pub(super) fn get_or_declare_memcpy(&self) -> FunctionValue<'ctx> {
        if let Some(f) = self.module.get_function("memcpy") {
            return f;
        }
        let ptr_ty = self.context.ptr_type(AddressSpace::default());
        let i64_ty = self.context.i64_type();
        // void *memcpy(void *dest, const void *src, size_t n)
        let memcpy_ty = ptr_ty.fn_type(&[ptr_ty.into(), ptr_ty.into(), i64_ty.into()], false);
        self.module.add_function("memcpy", memcpy_ty, None)
    }

    pub(super) fn get_or_declare_fopen(&self) -> FunctionValue<'ctx> {
        if let Some(f) = self.module.get_function("fopen") {
            return f;
        }
        let ptr_ty = self.context.ptr_type(AddressSpace::default());
        let fopen_ty = ptr_ty.fn_type(&[ptr_ty.into(), ptr_ty.into()], false);
        self.module.add_function("fopen", fopen_ty, None)
    }

    pub(super) fn get_or_declare_fclose(&self) -> FunctionValue<'ctx> {
        if let Some(f) = self.module.get_function("fclose") {
            return f;
        }
        let ptr_ty = self.context.ptr_type(AddressSpace::default());
        let i32_ty = self.context.i32_type();
        let fclose_ty = i32_ty.fn_type(&[ptr_ty.into()], false);
        self.module.add_function("fclose", fclose_ty, None)
    }

    pub(super) fn get_or_declare_fwrite(&self) -> FunctionValue<'ctx> {
        if let Some(f) = self.module.get_function("fwrite") {
            return f;
        }
        let ptr_ty = self.context.ptr_type(AddressSpace::default());
        let i64_ty = self.context.i64_type();
        // size_t fwrite(const void *ptr, size_t size, size_t nmemb, FILE *stream)
        let fwrite_ty = i64_ty.fn_type(
            &[ptr_ty.into(), i64_ty.into(), i64_ty.into(), ptr_ty.into()],
            false,
        );
        self.module.add_function("fwrite", fwrite_ty, None)
    }

    pub(super) fn get_or_declare_fgetc(&self) -> FunctionValue<'ctx> {
        if let Some(f) = self.module.get_function("fgetc") {
            return f;
        }
        let ptr_ty = self.context.ptr_type(AddressSpace::default());
        let i32_ty = self.context.i32_type();
        let fgetc_ty = i32_ty.fn_type(&[ptr_ty.into()], false);
        self.module.add_function("fgetc", fgetc_ty, None)
    }

    pub(super) fn get_or_declare_exit(&self) -> FunctionValue<'ctx> {
        if let Some(f) = self.module.get_function("exit") {
            return f;
        }
        let i32_ty = self.context.i32_type();
        let exit_ty = self.context.void_type().fn_type(&[i32_ty.into()], false);
        self.module.add_function("exit", exit_ty, None)
    }

    pub(super) fn get_or_declare_fgets(&self) -> FunctionValue<'ctx> {
        if let Some(f) = self.module.get_function("fgets") {
            return f;
        }
        let ptr_ty = self.context.ptr_type(AddressSpace::default());
        let i32_ty = self.context.i32_type();
        let fgets_ty = ptr_ty.fn_type(&[ptr_ty.into(), i32_ty.into(), ptr_ty.into()], false);
        self.module.add_function("fgets", fgets_ty, None)
    }

    pub(super) fn get_or_declare_stdin(&self) -> GlobalValue<'ctx> {
        if let Some(g) = self.module.get_global("stdin") {
            return g;
        }
        let ptr_ty = self.context.ptr_type(AddressSpace::default());
        self.module.add_global(ptr_ty, None, "stdin")
    }

    pub(super) fn get_or_declare_strlen(&self) -> FunctionValue<'ctx> {
        if let Some(f) = self.module.get_function("strlen") {
            return f;
        }
        let ptr_ty = self.context.ptr_type(AddressSpace::default());
        let i64_ty = self.context.i64_type();
        let strlen_ty = i64_ty.fn_type(&[ptr_ty.into()], false);
        self.module.add_function("strlen", strlen_ty, None)
    }

    pub(super) fn get_or_declare_atoi(&self) -> FunctionValue<'ctx> {
        if let Some(f) = self.module.get_function("atoi") {
            return f;
        }
        let ptr_ty = self.context.ptr_type(AddressSpace::default());
        let i32_ty = self.context.i32_type();
        let atoi_ty = i32_ty.fn_type(&[ptr_ty.into()], false);
        self.module.add_function("atoi", atoi_ty, None)
    }

    pub(super) fn get_or_declare_snprintf(&self) -> FunctionValue<'ctx> {
        if let Some(f) = self.module.get_function("snprintf") {
            return f;
        }
        let ptr_ty = self.context.ptr_type(AddressSpace::default());
        let i32_ty = self.context.i32_type();
        let i64_ty = self.context.i64_type();
        let snprintf_ty = i32_ty.fn_type(&[ptr_ty.into(), i64_ty.into(), ptr_ty.into()], true);
        self.module.add_function("snprintf", snprintf_ty, None)
    }

    pub(super) fn get_or_declare_usleep(&self) -> FunctionValue<'ctx> {
        if let Some(f) = self.module.get_function("usleep") {
            return f;
        }
        let i32_ty = self.context.i32_type();
        let usleep_ty = i32_ty.fn_type(&[i32_ty.into()], false);
        self.module.add_function("usleep", usleep_ty, None)
    }

    pub(super) fn get_or_declare_memcmp(&self) -> FunctionValue<'ctx> {
        if let Some(f) = self.module.get_function("memcmp") {
            return f;
        }
        let ptr_ty = self.context.ptr_type(AddressSpace::default());
        let i32_ty = self.context.i32_type();
        let i64_ty = self.context.i64_type();
        // int memcmp(const void *s1, const void *s2, size_t n)
        let memcmp_ty = i32_ty.fn_type(&[ptr_ty.into(), ptr_ty.into(), i64_ty.into()], false);
        self.module.add_function("memcmp", memcmp_ty, None)
    }

    // Stack trace
    pub(super) fn get_or_declare_ny_trace_push(&self) -> FunctionValue<'ctx> {
        if let Some(f) = self.module.get_function("ny_trace_push") { return f; }
        let ptr_ty = self.context.ptr_type(AddressSpace::default());
        let fn_ty = self.context.void_type().fn_type(&[ptr_ty.into()], false);
        self.module.add_function("ny_trace_push", fn_ty, None)
    }

    pub(super) fn get_or_declare_ny_trace_pop(&self) -> FunctionValue<'ctx> {
        if let Some(f) = self.module.get_function("ny_trace_pop") { return f; }
        let fn_ty = self.context.void_type().fn_type(&[], false);
        self.module.add_function("ny_trace_pop", fn_ty, None)
    }

    pub(super) fn get_or_declare_ny_trace_print(&self) -> FunctionValue<'ctx> {
        if let Some(f) = self.module.get_function("ny_trace_print") { return f; }
        let fn_ty = self.context.void_type().fn_type(&[], false);
        self.module.add_function("ny_trace_print", fn_ty, None)
    }

    // JSON runtime declarations
    pub(super) fn get_or_declare_ny_json_parse(&self) -> FunctionValue<'ctx> {
        if let Some(f) = self.module.get_function("ny_json_parse") { return f; }
        let ptr_ty = self.context.ptr_type(AddressSpace::default());
        let i64_ty = self.context.i64_type();
        let fn_ty = ptr_ty.fn_type(&[ptr_ty.into(), i64_ty.into()], false);
        self.module.add_function("ny_json_parse", fn_ty, None)
    }

    pub(super) fn get_or_declare_ny_json_type(&self) -> FunctionValue<'ctx> {
        if let Some(f) = self.module.get_function("ny_json_type") { return f; }
        let ptr_ty = self.context.ptr_type(AddressSpace::default());
        let i32_ty = self.context.i32_type();
        let fn_ty = i32_ty.fn_type(&[ptr_ty.into()], false);
        self.module.add_function("ny_json_type", fn_ty, None)
    }

    pub(super) fn get_or_declare_ny_json_get_int(&self) -> FunctionValue<'ctx> {
        if let Some(f) = self.module.get_function("ny_json_get_int") { return f; }
        let ptr_ty = self.context.ptr_type(AddressSpace::default());
        let i64_ty = self.context.i64_type();
        let fn_ty = i64_ty.fn_type(&[ptr_ty.into(), ptr_ty.into(), i64_ty.into()], false);
        self.module.add_function("ny_json_get_int", fn_ty, None)
    }

    pub(super) fn get_or_declare_ny_json_get_float(&self) -> FunctionValue<'ctx> {
        if let Some(f) = self.module.get_function("ny_json_get_float") { return f; }
        let ptr_ty = self.context.ptr_type(AddressSpace::default());
        let f64_ty = self.context.f64_type();
        let i64_ty = self.context.i64_type();
        let fn_ty = f64_ty.fn_type(&[ptr_ty.into(), ptr_ty.into(), i64_ty.into()], false);
        self.module.add_function("ny_json_get_float", fn_ty, None)
    }

    pub(super) fn get_or_declare_ny_json_get_str(&self) -> FunctionValue<'ctx> {
        if let Some(f) = self.module.get_function("ny_json_get_str") { return f; }
        let ptr_ty = self.context.ptr_type(AddressSpace::default());
        let i64_ty = self.context.i64_type();
        // returns ptr, takes (obj, key_ptr, key_len, &out_len)
        let fn_ty = ptr_ty.fn_type(&[ptr_ty.into(), ptr_ty.into(), i64_ty.into(), ptr_ty.into()], false);
        self.module.add_function("ny_json_get_str", fn_ty, None)
    }

    pub(super) fn get_or_declare_ny_json_get_bool(&self) -> FunctionValue<'ctx> {
        if let Some(f) = self.module.get_function("ny_json_get_bool") { return f; }
        let ptr_ty = self.context.ptr_type(AddressSpace::default());
        let i32_ty = self.context.i32_type();
        let i64_ty = self.context.i64_type();
        let fn_ty = i32_ty.fn_type(&[ptr_ty.into(), ptr_ty.into(), i64_ty.into()], false);
        self.module.add_function("ny_json_get_bool", fn_ty, None)
    }

    pub(super) fn get_or_declare_ny_json_len(&self) -> FunctionValue<'ctx> {
        if let Some(f) = self.module.get_function("ny_json_len") { return f; }
        let ptr_ty = self.context.ptr_type(AddressSpace::default());
        let i64_ty = self.context.i64_type();
        let fn_ty = i64_ty.fn_type(&[ptr_ty.into()], false);
        self.module.add_function("ny_json_len", fn_ty, None)
    }

    pub(super) fn get_or_declare_ny_json_arr_get(&self) -> FunctionValue<'ctx> {
        if let Some(f) = self.module.get_function("ny_json_arr_get") { return f; }
        let ptr_ty = self.context.ptr_type(AddressSpace::default());
        let i64_ty = self.context.i64_type();
        let fn_ty = ptr_ty.fn_type(&[ptr_ty.into(), i64_ty.into()], false);
        self.module.add_function("ny_json_arr_get", fn_ty, None)
    }

    pub(super) fn get_or_declare_ny_json_free(&self) -> FunctionValue<'ctx> {
        if let Some(f) = self.module.get_function("ny_json_free") { return f; }
        let ptr_ty = self.context.ptr_type(AddressSpace::default());
        let fn_ty = self.context.void_type().fn_type(&[ptr_ty.into()], false);
        self.module.add_function("ny_json_free", fn_ty, None)
    }

    pub(super) fn get_or_declare_ny_remove_file(&self) -> FunctionValue<'ctx> {
        if let Some(f) = self.module.get_function("ny_remove_file") {
            return f;
        }
        let ptr_ty = self.context.ptr_type(AddressSpace::default());
        let i64_ty = self.context.i64_type();
        let i32_ty = self.context.i32_type();
        let fn_ty = i32_ty.fn_type(&[ptr_ty.into(), i64_ty.into()], false);
        self.module.add_function("ny_remove_file", fn_ty, None)
    }

    pub(super) fn get_or_declare_ny_read_file(&self) -> FunctionValue<'ctx> {
        if let Some(f) = self.module.get_function("ny_read_file") {
            return f;
        }
        let ptr_ty = self.context.ptr_type(AddressSpace::default());
        let i64_ty = self.context.i64_type();
        let fn_ty = ptr_ty.fn_type(&[ptr_ty.into(), i64_ty.into(), ptr_ty.into()], false);
        self.module.add_function("ny_read_file", fn_ty, None)
    }

    pub(super) fn get_or_declare_ny_write_file(&self) -> FunctionValue<'ctx> {
        if let Some(f) = self.module.get_function("ny_write_file") {
            return f;
        }
        let ptr_ty = self.context.ptr_type(AddressSpace::default());
        let i64_ty = self.context.i64_type();
        let i32_ty = self.context.i32_type();
        let fn_ty =
            i32_ty.fn_type(&[ptr_ty.into(), i64_ty.into(), ptr_ty.into(), i64_ty.into()], false);
        self.module.add_function("ny_write_file", fn_ty, None)
    }

    pub(super) fn get_or_declare_ny_float_to_str(&self) -> FunctionValue<'ctx> {
        if let Some(f) = self.module.get_function("ny_float_to_str") {
            return f;
        }
        let ptr_ty = self.context.ptr_type(AddressSpace::default());
        let f64_ty = self.context.f64_type();
        let fn_ty = ptr_ty.fn_type(&[f64_ty.into(), ptr_ty.into()], false);
        self.module.add_function("ny_float_to_str", fn_ty, None)
    }

    pub(super) fn get_or_declare_ny_str_to_float(&self) -> FunctionValue<'ctx> {
        if let Some(f) = self.module.get_function("ny_str_to_float") {
            return f;
        }
        let ptr_ty = self.context.ptr_type(AddressSpace::default());
        let f64_ty = self.context.f64_type();
        let i64_ty = self.context.i64_type();
        let fn_ty = f64_ty.fn_type(&[ptr_ty.into(), i64_ty.into()], false);
        self.module.add_function("ny_str_to_float", fn_ty, None)
    }

    pub(super) fn get_or_declare_ny_clock_ms(&self) -> FunctionValue<'ctx> {
        if let Some(f) = self.module.get_function("ny_clock_ms") {
            return f;
        }
        let i64_ty = self.context.i64_type();
        let fn_ty = i64_ty.fn_type(&[], false);
        self.module.add_function("ny_clock_ms", fn_ty, None)
    }

    pub(super) fn get_or_declare_ny_str_split(&self) -> FunctionValue<'ctx> {
        if let Some(f) = self.module.get_function("ny_str_split") {
            return f;
        }
        let ptr_ty = self.context.ptr_type(AddressSpace::default());
        let i64_ty = self.context.i64_type();
        // NyStrSlice *ny_str_split(ptr hay, i64 hay_len, ptr delim, i64 delim_len, ptr out_count)
        let fn_ty = ptr_ty.fn_type(
            &[ptr_ty.into(), i64_ty.into(), ptr_ty.into(), i64_ty.into(), ptr_ty.into()],
            false,
        );
        self.module.add_function("ny_str_split", fn_ty, None)
    }

    pub(super) fn get_or_declare_ny_str_replace(&self) -> FunctionValue<'ctx> {
        if let Some(f) = self.module.get_function("ny_str_replace") {
            return f;
        }
        let ptr_ty = self.context.ptr_type(AddressSpace::default());
        let i64_ty = self.context.i64_type();
        // char *ny_str_replace(ptr hay, i64 hay_len, ptr old, i64 old_len,
        //                      ptr new, i64 new_len, ptr out_len)
        let fn_ty = ptr_ty.fn_type(
            &[
                ptr_ty.into(),
                i64_ty.into(),
                ptr_ty.into(),
                i64_ty.into(),
                ptr_ty.into(),
                i64_ty.into(),
                ptr_ty.into(),
            ],
            false,
        );
        self.module.add_function("ny_str_replace", fn_ty, None)
    }

    pub(super) fn get_or_declare_toupper(&self) -> FunctionValue<'ctx> {
        if let Some(f) = self.module.get_function("toupper") {
            return f;
        }
        let i32_ty = self.context.i32_type();
        let toupper_ty = i32_ty.fn_type(&[i32_ty.into()], false);
        self.module.add_function("toupper", toupper_ty, None)
    }

    pub(super) fn get_or_declare_tolower(&self) -> FunctionValue<'ctx> {
        if let Some(f) = self.module.get_function("tolower") {
            return f;
        }
        let i32_ty = self.context.i32_type();
        let tolower_ty = i32_ty.fn_type(&[i32_ty.into()], false);
        self.module.add_function("tolower", tolower_ty, None)
    }

    pub(super) fn get_or_declare_fprintf(&self) -> FunctionValue<'ctx> {
        if let Some(f) = self.module.get_function("fprintf") {
            return f;
        }
        let ptr_ty = self.context.ptr_type(AddressSpace::default());
        let i32_ty = self.context.i32_type();
        let fprintf_ty = i32_ty.fn_type(&[ptr_ty.into(), ptr_ty.into()], true);
        self.module.add_function("fprintf", fprintf_ty, None)
    }

    pub(super) fn get_or_declare_stderr(&self) -> GlobalValue<'ctx> {
        if let Some(g) = self.module.get_global("stderr") {
            return g;
        }
        let ptr_ty = self.context.ptr_type(AddressSpace::default());
        self.module.add_global(ptr_ty, None, "stderr")
    }
}
