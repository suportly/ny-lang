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
