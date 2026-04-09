// tests/slm_test.rs

use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::TempDir;

fn compile_and_run(fixture: &str) -> (i32, String) {
    let tmp = TempDir::new().unwrap();
    let output = tmp.path().join("output");

    let mut cmd = Command::cargo_bin("ny").unwrap();
    cmd.arg("build")
        .arg(format!("tests/fixtures/valid/{}", fixture))
        .arg("-o")
        .arg(&output);

    cmd.assert().success();

    let output = std::process::Command::new(&output)
        .output()
        .expect("failed to run compiled binary");

    let exit_code = output.status.code().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    (exit_code, stdout)
}

#[test]
fn test_slm_forward_pass() {
    let (exit_code, stdout) = compile_and_run("slm_forward_pass.ny");
    assert_eq!(exit_code, 42);
    assert!(stdout.contains("SLM forward pass successful"), "stdout: {}", stdout);
}
