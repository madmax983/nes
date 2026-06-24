use std::process::Command;

#[test]
fn build_homebrew_rom_help() {
    let output = Command::new("cargo")
        .args(["run", "--bin", "build_homebrew_rom", "--", "--help"])
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Usage: build_homebrew_rom [--out <path>]"));
}

#[test]
fn build_homebrew_rom_invalid_arg() {
    let output = Command::new("cargo")
        .args(["run", "--bin", "build_homebrew_rom", "--", "--invalid"])
        .output()
        .unwrap();

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("unknown argument '--invalid'"));
}

#[test]
fn build_homebrew_rom_out() {
    let output = Command::new("cargo")
        .args(["run", "--bin", "build_homebrew_rom", "--", "--out", "test_rom.nes"])
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Success"));
    assert!(stdout.contains("test_rom.nes"));

    // Cleanup
    let _ = std::fs::remove_file("test_rom.nes");
}
