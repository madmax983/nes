use std::process::Command;

#[test]
fn test_build_homebrew_rom_cli_help() {
    let output = Command::new(env!("CARGO_BIN_EXE_build_homebrew_rom"))
        .arg("--help")
        .output()
        .expect("Failed to execute build_homebrew_rom");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Usage: build_homebrew_rom"));
}

#[test]
fn test_build_homebrew_rom_cli_out() {
    let out_path = std::env::temp_dir().join(format!(
        "test_homebrew_{}.nes",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));

    let output = Command::new(env!("CARGO_BIN_EXE_build_homebrew_rom"))
        .args(["--out", out_path.to_str().unwrap()])
        .output()
        .expect("Failed to execute build_homebrew_rom");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("test_homebrew"));
    assert!(out_path.exists());
    let _ = std::fs::remove_file(&out_path);
}

#[test]
fn test_build_homebrew_rom_cli_out_missing_arg() {
    let output = Command::new(env!("CARGO_BIN_EXE_build_homebrew_rom"))
        .arg("--out")
        .output()
        .expect("Failed to execute build_homebrew_rom");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("missing value after --out"));
}

#[test]
fn test_build_homebrew_rom_cli_unknown_arg() {
    let output = Command::new(env!("CARGO_BIN_EXE_build_homebrew_rom"))
        .arg("--invalid")
        .output()
        .expect("Failed to execute build_homebrew_rom");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("unknown argument"));
}
