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
fn golden_capture_with_invalid_flag_prints_error() {
    let output = Command::new(golden_capture_bin())
        .arg("--invalid")
        .output()
        .expect("run bbbradsmith_golden_capture");

    assert!(!output.status.success(), "should fail on invalid flag");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("unknown argument '--invalid'"));
}

#[test]
fn golden_capture_with_missing_config_fails() {
    let output = Command::new(golden_capture_bin())
        .args(["--config", "nonexistent_config.toml"])
        .output()
        .expect("run bbbradsmith_golden_capture");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("failed to read config"));
}

#[test]
fn golden_capture_with_empty_suite_dir_fails() {
    let temp_dir_path = std::env::temp_dir().join(format!(
        "test_{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&temp_dir_path).unwrap();
    let config_path = temp_dir_path.as_path().join("nes.toml");
    let suite_dir = temp_dir_path.as_path().join("suite");
    let golden_dir = temp_dir_path.as_path().join("golden");

    std::fs::create_dir(&suite_dir).unwrap();

    let config_content = format!(
        r#"
[roms]
bbbradsmith_audio_suite_dir = "{}"
bbbradsmith_audio_golden_dir = "{}"
"#,
        suite_dir.display(),
        golden_dir.display()
    );

    std::fs::write(&config_path, config_content).unwrap();

    let output = Command::new(golden_capture_bin())
        .args(["--config", config_path.to_str().unwrap()])
        .output()
        .expect("run bbbradsmith_golden_capture");

    assert!(!output.status.success(), "should fail on empty suite dir");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("no .nes files found in suite directory"));
}

#[test]
fn golden_capture_with_missing_suite_dir_fails() {
    let temp_dir_path = std::env::temp_dir().join(format!(
        "test_{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&temp_dir_path).unwrap();
    let config_path = temp_dir_path.as_path().join("nes.toml");
    let suite_dir = temp_dir_path.as_path().join("suite_does_not_exist");
    let golden_dir = temp_dir_path.as_path().join("golden");

    let config_content = format!(
        r#"
[roms]
bbbradsmith_audio_suite_dir = "{}"
bbbradsmith_audio_golden_dir = "{}"
"#,
        suite_dir.display(),
        golden_dir.display()
    );

    std::fs::write(&config_path, config_content).unwrap();

    let output = Command::new(golden_capture_bin())
        .args(["--config", config_path.to_str().unwrap()])
        .output()
        .expect("run bbbradsmith_golden_capture");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("does not exist or is not a directory"));
}
