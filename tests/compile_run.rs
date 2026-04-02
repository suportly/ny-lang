use assert_cmd::Command;
use predicates::prelude::*;
use std::process;
use tempfile::TempDir;

fn compile_and_run(fixture: &str) -> i32 {
    let tmp = TempDir::new().unwrap();
    let output = tmp.path().join("output");

    let mut cmd = Command::cargo_bin("ny").unwrap();
    cmd.arg("build")
        .arg(format!("tests/fixtures/valid/{}", fixture))
        .arg("-o")
        .arg(&output);

    cmd.assert().success();

    let status = process::Command::new(&output)
        .status()
        .expect("failed to run compiled binary");

    status.code().unwrap()
}

#[test]
fn test_return_42() {
    assert_eq!(compile_and_run("return_42.lnge"), 42);
}

#[test]
fn test_function_call() {
    assert_eq!(compile_and_run("function_call.lnge"), 42);
}

#[test]
fn test_arithmetic() {
    assert_eq!(compile_and_run("arithmetic.lnge"), 14); // 2 + 3 * 4
}

#[test]
fn test_fibonacci() {
    assert_eq!(compile_and_run("fibonacci.lnge"), 55); // fib(10)
}

#[test]
fn test_variables() {
    assert_eq!(compile_and_run("variables.lnge"), 45); // sum 0..9
}

#[test]
fn test_control_flow() {
    assert_eq!(compile_and_run("control_flow.lnge"), 7); // abs(-7)
}

#[test]
fn test_expression_body() {
    assert_eq!(compile_and_run("expression_body.lnge"), 25); // 5 * 5
}

// Phase 2 tests

#[test]
fn test_for_range() {
    assert_eq!(compile_and_run("for_range.ny"), 45); // sum 0..10
}

#[test]
fn test_break_continue() {
    assert_eq!(compile_and_run("break_continue.ny"), 20); // 0+2+4+6+8
}

#[test]
fn test_arrays() {
    assert_eq!(compile_and_run("arrays.ny"), 150); // 10+20+30+40+50
}

#[test]
fn test_structs() {
    assert_eq!(compile_and_run("structs.ny"), 11); // dot(3,4).(1,2) = 3+8
}

#[test]
fn test_pointers() {
    assert_eq!(compile_and_run("pointers.ny"), 20); // swap(10,20) → x=20
}

#[test]
fn test_inference() {
    assert_eq!(compile_and_run("inference.ny"), 15); // 5+10
}

#[test]
fn test_hello_print() {
    let tmp = TempDir::new().unwrap();
    let output = tmp.path().join("output");

    let mut cmd = Command::cargo_bin("ny").unwrap();
    cmd.arg("build")
        .arg("tests/fixtures/valid/hello_print.ny")
        .arg("-o")
        .arg(&output);
    cmd.assert().success();

    let out = process::Command::new(&output)
        .output()
        .expect("failed to run");

    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("Hello, Ny!"), "stdout: {}", stdout);
    assert!(stdout.contains("42"), "stdout: {}", stdout);
    assert!(stdout.contains("true"), "stdout: {}", stdout);
}

// Phase 3 tests

#[test]
fn test_compound_assign() {
    assert_eq!(compile_and_run("compound_assign.ny"), 45);
}

#[test]
fn test_bitwise() {
    assert_eq!(compile_and_run("bitwise.ny"), 39); // 15+16+8
}

#[test]
fn test_casting() {
    assert_eq!(compile_and_run("casting.ny"), 46); // 3+42+1
}

#[test]
fn test_block_comments() {
    assert_eq!(compile_and_run("block_comments.ny"), 42);
}

// Phase 4 tests

#[test]
fn test_enums() {
    assert_eq!(compile_and_run("enums.ny"), 2); // Color::Green → 2
}

#[test]
fn test_match_expr() {
    assert_eq!(compile_and_run("match_expr.ny"), 119); // describe(1)+describe(5) = 20+99
}

#[test]
fn test_tuples() {
    assert_eq!(compile_and_run("tuples.ny"), 3); // 10/3 = 3
}

// Phase 5 tests

#[test]
fn test_defer_alloc() {
    assert_eq!(compile_and_run("defer_alloc.ny"), 42);
}

#[test]
fn test_defer_lifo() {
    assert_eq!(compile_and_run("defer_lifo.ny"), 42);
}

#[test]
fn test_substr() {
    let tmp = TempDir::new().unwrap();
    let output = tmp.path().join("output");
    Command::cargo_bin("ny")
        .unwrap()
        .args(["build", "tests/fixtures/valid/substr_test.ny", "-o"])
        .arg(&output)
        .assert()
        .success();
    let out = process::Command::new(&output)
        .output()
        .expect("failed to run");
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("hello"), "stdout: {}", stdout);
    assert!(stdout.contains("world"), "stdout: {}", stdout);
    assert_eq!(out.status.code().unwrap(), 42);
}

// Phase 6 tests

#[test]
fn test_loop_stmt() {
    assert_eq!(compile_and_run("loop_stmt.ny"), 45);
}

#[test]
fn test_tagged_union() {
    assert_eq!(compile_and_run("tagged_union.ny"), 42);
}

#[test]
fn test_multi_field_enum() {
    assert_eq!(compile_and_run("multi_field_enum.ny"), 47);
}

// Phase 7 tests

#[test]
fn test_impl_block() {
    assert_eq!(compile_and_run("impl_block.ny"), 52);
}

// Phase 8 tests

#[test]
fn test_traits() {
    assert_eq!(compile_and_run("traits.ny"), 52);
}

// Phase 9 tests

#[test]
fn test_slices() {
    assert_eq!(compile_and_run("slices.ny"), 23);
}

// Phase G tests (for-in)

#[test]
fn test_for_in() {
    assert_eq!(compile_and_run("for_in.ny"), 42);
}

// Phase F tests (Vec)

#[test]
fn test_vec() {
    assert_eq!(compile_and_run("vec_test.ny"), 57);
}

#[test]
fn test_generic_enum() {
    assert_eq!(compile_and_run("generic_enum.ny"), 42);
}

#[test]
fn test_numeric_coerce() {
    assert_eq!(compile_and_run("numeric_coerce.ny"), 42);
}

// Stdlib tests

#[test]
fn test_stdlib() {
    assert_eq!(compile_and_run("stdlib_test.ny"), 42);
}

// Phase 15: Concurrency tests

#[test]
fn test_channel() {
    assert_eq!(compile_and_run("channel.ny"), 42);
}

#[test]
fn test_threadpool() {
    assert_eq!(compile_and_run("threadpool.ny"), 42);
}

#[test]
fn test_par_map() {
    assert_eq!(compile_and_run("par_map.ny"), 42);
}

#[test]
fn test_simd_dotprod() {
    assert_eq!(compile_and_run("simd_dotprod.ny"), 42);
}

#[test]
fn test_threads() {
    assert_eq!(compile_and_run("threads.ny"), 42);
}

#[test]
fn test_to_str() {
    let tmp = TempDir::new().unwrap();
    let output = tmp.path().join("output");
    Command::cargo_bin("ny").unwrap()
        .args(["build", "tests/fixtures/valid/to_str_test.ny", "-o"])
        .arg(&output).assert().success();
    let out = process::Command::new(&output).output().expect("failed");
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("value=42"), "stdout: {}", stdout);
    assert_eq!(out.status.code().unwrap(), 42);
}

#[test]
fn test_while_let() {
    assert_eq!(compile_and_run("while_let.ny"), 42);
}

#[test]
fn test_simd() {
    assert_eq!(compile_and_run("simd.ny"), 42);
}

#[test]
fn test_generic_struct() {
    assert_eq!(compile_and_run("generic_struct.ny"), 42);
}

#[test]
fn test_arena() {
    assert_eq!(compile_and_run("arena.ny"), 42);
}

#[test]
fn test_closure() {
    assert_eq!(compile_and_run("closure.ny"), 42);
}

#[test]
fn test_multi_closure() {
    assert_eq!(compile_and_run("multi_closure.ny"), 50);
}

#[test]
fn test_ptr_arith() {
    assert_eq!(compile_and_run("ptr_arith.ny"), 42);
}

#[test]
fn test_hashmap() {
    assert_eq!(compile_and_run("hashmap.ny"), 11);
}

#[test]
fn test_void_fn() {
    assert_eq!(compile_and_run("void_fn.ny"), 42);
}

#[test]
fn test_vec_f64() {
    assert_eq!(compile_and_run("vec_f64.ny"), 10);
}

// Phase E tests (generics)

#[test]
fn test_generics() {
    assert_eq!(compile_and_run("generics.ny"), 42);
}

// if let test

#[test]
fn test_if_let() {
    assert_eq!(compile_and_run("if_let.ny"), 42);
}

// Phase H tests (extern FFI)

#[test]
fn test_extern_ffi() {
    assert_eq!(compile_and_run("extern_ffi.ny"), 42);
}

// Phase D tests (modules)

#[test]
fn test_use_module() {
    assert_eq!(compile_and_run("use_module.ny"), 42);
}

// Phase C tests (? operator)

#[test]
fn test_try_operator() {
    assert_eq!(compile_and_run("try_operator.ny"), 42);
}

// Phase B tests (stdin/to_str)

#[test]
fn test_int_to_str() {
    let tmp = TempDir::new().unwrap();
    let output = tmp.path().join("output");
    Command::cargo_bin("ny")
        .unwrap()
        .args(["build", "tests/fixtures/valid/int_to_str.ny", "-o"])
        .arg(&output)
        .assert()
        .success();
    let out = process::Command::new(&output)
        .output()
        .expect("failed to run");
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("42"), "stdout: {}", stdout);
    assert!(stdout.contains("value=42"), "stdout: {}", stdout);
    assert_eq!(out.status.code().unwrap(), 5);
}

// Phase 11 tests

#[test]
fn test_lambda() {
    assert_eq!(compile_and_run("lambda.ny"), 42);
}

// Phase 10 tests

#[test]
fn test_file_io() {
    assert_eq!(compile_and_run("file_io.ny"), 42);
}

// Phase 11 tests

#[test]
fn test_unsafe_ptr() {
    assert_eq!(compile_and_run("unsafe_ptr.ny"), 42);
}

#[test]
fn test_string_ops() {
    let tmp = TempDir::new().unwrap();
    let output = tmp.path().join("output");
    Command::cargo_bin("ny")
        .unwrap()
        .args(["build", "tests/fixtures/valid/string_ops.ny", "-o"])
        .arg(&output)
        .assert()
        .success();
    let out = process::Command::new(&output)
        .output()
        .expect("failed to run");
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("hello world"), "stdout: {}", stdout);
    assert!(stdout.contains("5"), "stdout: {}", stdout);
    assert_eq!(out.status.code().unwrap(), 42);
}
