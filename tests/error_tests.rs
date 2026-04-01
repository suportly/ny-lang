use assert_cmd::Command;
use predicates::prelude::*;

fn compile_invalid(fixture: &str) -> assert_cmd::assert::Assert {
    let mut cmd = Command::cargo_bin("ny").unwrap();
    cmd.arg("build")
        .arg(format!("tests/fixtures/invalid/{}", fixture))
        .arg("-o")
        .arg("/tmp/ny_test_bad");

    cmd.assert()
}

#[test]
fn test_type_mismatch() {
    compile_invalid("type_mismatch.lnge")
        .failure()
        .code(1)
        .stderr(predicate::str::contains("expected 'i32', found 'bool'"));
}

#[test]
fn test_undeclared_variable() {
    compile_invalid("undeclared_var.lnge")
        .failure()
        .code(1)
        .stderr(predicate::str::contains("undeclared variable 'y'"));
}

#[test]
fn test_immutable_assign() {
    compile_invalid("immutable_assign.lnge")
        .failure()
        .code(1)
        .stderr(predicate::str::contains(
            "cannot assign to immutable variable",
        ));
}

#[test]
fn test_empty_file() {
    compile_invalid("empty.lnge")
        .failure()
        .code(1)
        .stderr(predicate::str::contains("no 'main' function found"));
}

#[test]
fn test_nonexistent_file() {
    let mut cmd = Command::cargo_bin("ny").unwrap();
    cmd.arg("build").arg("nonexistent.lnge");

    cmd.assert()
        .failure()
        .code(2)
        .stderr(predicate::str::contains("No such file or directory"));
}
