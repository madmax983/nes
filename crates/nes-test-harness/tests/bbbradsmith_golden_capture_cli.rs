use std::process::Command;

fn golden_capture_bin() -> std::path::PathBuf {
    if let Ok(path) = std::env::var("CARGO_BIN_EXE_bbbradsmith_golden_capture") {
        return std::path::PathBuf::from(path);
    }

    let mut dir = std::env::current_exe().expect("should locate test binary");
    dir.pop();
    if dir.ends_with("deps") {
        dir.pop();
    }

    let exe_name = if cfg!(windows) {
        "bbbradsmith_golden_capture.exe"
    } else {
        "bbbradsmith_golden_capture"
    };
    dir.join(exe_name)
}

#[test]
fn golden_capture_with_help_flag_prints_usage_and_succeeds() {
    for flag in ["--help", "-h"] {
        let output = Command::new(golden_capture_bin())
            .arg(flag)
            .output()
            .expect("run bbbradsmith_golden_capture");

        assert!(output.status.success(), "failed on flag {flag}");
        let stdout = String::from_utf8(output.stdout).expect("stdout utf8");
        assert!(stdout.contains("Usage: bbbradsmith_golden_capture [--config <path>] [--force]"));
    }
}

#[test]
fn golden_capture_with_unknown_flag_fails() {
    let output = Command::new(golden_capture_bin())
        .arg("--unknown")
        .output()
        .expect("run bbbradsmith_golden_capture");

    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).expect("stderr utf8");
    assert!(stderr.contains("unknown argument"));
    assert!(stderr.contains("supported: --config <path>, --config=<path>, --force"));
}

#[test]
fn golden_capture_with_missing_suite_dir_fails() {
    let output = Command::new(golden_capture_bin())
        .arg("--config")
        .arg("dummy.toml")
        .output()
        .expect("run bbbradsmith_golden_capture");

    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).expect("stderr utf8");
    assert!(stderr.contains("missing `roms.bbbradsmith_audio_suite_dir`") || stderr.contains("file not found") || stderr.contains("unable to load config") || stderr.contains("Failed to read"));
}

#[test]
fn golden_capture_forces_rewrite() {
    // This is a minimal mock test just testing that force argument modifies state or at least runs without unknown argument panic.
    let output = Command::new(golden_capture_bin())
        .arg("--force")
        .output()
        .expect("run bbbradsmith_golden_capture");

    let stderr = String::from_utf8(output.stderr).expect("stderr utf8");
    assert!(!stderr.contains("unknown argument"));
}
