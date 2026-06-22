use std::process::Command;
use std::fs;

fn build_homebrew_bin() -> std::path::PathBuf {
    if let Ok(path) = std::env::var("CARGO_BIN_EXE_build_homebrew_rom") {
        return std::path::PathBuf::from(path);
    }

    let mut dir = std::env::current_exe().expect("should locate test binary");
    dir.pop();
    if dir.ends_with("deps") {
        dir.pop();
    }

    let exe_name = if cfg!(windows) {
        "build_homebrew_rom.exe"
    } else {
        "build_homebrew_rom"
    };
    dir.join(exe_name)
}

#[test]
fn test_build_homebrew_rom_help() {
    let output = Command::new(build_homebrew_bin())
        .arg("--help")
        .output()
        .expect("Failed to execute build_homebrew_rom");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Usage: build_homebrew_rom"));
    assert!(stdout.contains("--out <path>"));
}

#[test]
fn test_build_homebrew_rom_out() {
    let out_path = "test_out_homebrew.nes";

    let output = Command::new(build_homebrew_bin())
        .arg("--out")
        .arg(out_path)
        .output()
        .expect("Failed to execute build_homebrew_rom");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Building Homebrew ROM..."));

    if std::path::Path::new(out_path).exists() {
        fs::remove_file(out_path).unwrap();
    }
}
