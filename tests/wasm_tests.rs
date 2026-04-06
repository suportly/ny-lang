use assert_cmd::Command;
use std::path::Path;
use tempfile::TempDir;

fn compile_to_wasm(fixture: &str) {
    let tmp = TempDir::new().unwrap();
    let output_path = tmp.path().join("output.wasm");

    let mut cmd = Command::cargo_bin("ny").unwrap();
    cmd.arg("build")
        .arg(format!("tests/fixtures/wasm/{}", fixture))
        .arg("--target")
        .arg("wasm32")
        .arg("-o")
        .arg(&output_path);

    cmd.assert().success();

    assert!(
        output_path.exists(),
        "wasm output file was not created"
    );

    // Optional: Validate wasm binary
    // This requires wasm-validate from wasm-tools or similar
    if let Ok(status) = std::process::Command::new("wasm-validate").arg(&output_path).status() {
        assert!(status.success(), "Generated .wasm file is not valid");
    } else {
        // wasm-validate not found, skip validation
        println!("warning: wasm-validate not found, skipping wasm validation");
    }
}

#[test]
fn test_simple_wasm_compilation() {
    compile_to_wasm("simple.ny");
}
