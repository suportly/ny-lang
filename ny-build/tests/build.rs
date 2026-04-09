use assert_cmd::prelude::*;
use std::process::Command;
use tempfile::tempdir;
use std::fs;
use std::path::Path;

fn create_dummy_project(dir: &Path, manifest_content: &str) {
    fs::write(dir.join("NyProject.toml"), manifest_content).unwrap();
    let src_dir = dir.join("src");
    fs::create_dir(&src_dir).unwrap();
    fs::write(src_dir.join("main.ny"), "fn main() {}").unwrap();
}

#[test]
fn test_build_command_success() {
    let dir = tempdir().unwrap();
    let manifest_content = r#"
[package]
name = "my-test-project"
version = "0.1.0"
"#;
    create_dummy_project(dir.path(), manifest_content);

    let mut cmd = Command::cargo_bin("ny-build").unwrap();
    cmd.arg("build")
        .arg("--manifest-path")
        .arg(dir.path().join("NyProject.toml"))
        .current_dir(dir.path());

    cmd.assert().success();
    
    let build_dir = dir.path().join("build");
    assert!(build_dir.exists());
    assert!(build_dir.join("src").join("main.ny").exists());
}

#[test]
fn test_build_command_manifest_not_found() {
    let dir = tempdir().unwrap();
    
    let mut cmd = Command::cargo_bin("ny-build").unwrap();
    cmd.arg("build")
        .arg("--manifest-path")
        .arg(dir.path().join("NonExistent.toml"))
        .current_dir(dir.path());

    cmd.assert()
        .failure()
        .stderr(predicates::str::contains("Manifest file not found"));
}
