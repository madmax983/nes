use std::process::Command;

fn build_homebrew_rom_bin() -> std::path::PathBuf {
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
fn build_homebrew_rom_with_help_flag_prints_usage_and_succeeds() {
    for flag in ["--help", "-h"] {
        let output = Command::new(build_homebrew_rom_bin())
            .arg(flag)
            .output()
            .expect("run build_homebrew_rom");

        assert!(output.status.success(), "failed on flag {flag}");
        let stdout = String::from_utf8(output.stdout).expect("stdout utf8");
        assert!(stdout.contains("Usage: build_homebrew_rom [--out <path>]"));
    }
}

#[test]
fn build_homebrew_rom_with_out_flag_builds_and_succeeds() {
    let temp_dir = std::env::temp_dir().join("build_homebrew_rom_out_test");
    let _ = std::fs::remove_dir_all(&temp_dir);
    std::fs::create_dir_all(&temp_dir).unwrap();
    let out_path = temp_dir.join("test_homebrew.nes");

    let output = Command::new(build_homebrew_rom_bin())
        .arg("--out")
        .arg(&out_path)
        .output()
        .expect("run build_homebrew_rom");

    assert!(output.status.success(), "failed with output: {:?}", output);
    let stdout = String::from_utf8(output.stdout).expect("stdout utf8");
    assert!(stdout.contains("Building Homebrew ROM..."));
    assert!(stdout.contains("test_homebrew.nes"));

    assert!(out_path.exists());
    let _ = std::fs::remove_dir_all(&temp_dir);
}
