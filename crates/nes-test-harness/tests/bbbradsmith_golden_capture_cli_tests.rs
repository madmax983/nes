use std::process::Command;

#[test]
fn bbbradsmith_golden_capture_help() {
    let output = Command::new("cargo")
        .args(["run", "--bin", "bbbradsmith_golden_capture", "--", "--help"])
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Usage: bbbradsmith_golden_capture"));
}

#[test]
fn bbbradsmith_golden_capture_invalid_arg() {
    let output = Command::new("cargo")
        .args(["run", "--bin", "bbbradsmith_golden_capture", "--", "--invalid"])
        .output()
        .unwrap();

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("unknown argument '--invalid'"));
}
