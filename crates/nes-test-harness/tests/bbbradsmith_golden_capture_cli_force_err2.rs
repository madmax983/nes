use std::process::Command;
use std::fs;

#[test]
fn golden_capture_fails_if_run_returns_err() {
    let output = Command::new("cargo")
        .args(&["run", "--bin", "bbbradsmith_golden_capture", "--", "--config", "/does/not/exist/12345/nes.toml"])
        .output()
        .expect("Failed to execute process");

    assert!(!output.status.success());
    assert_eq!(output.status.code(), Some(1));
}
