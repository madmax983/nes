use std::process::Command;

fn train_smb_control_bin() -> std::path::PathBuf {
    if let Ok(path) = std::env::var("CARGO_BIN_EXE_train_smb_control") {
        return std::path::PathBuf::from(path);
    }
    let mut dir = std::env::current_exe().expect("should locate test binary");
    dir.pop();
    if dir.ends_with("deps") {
        dir.pop();
    }
    let exe_name = if cfg!(windows) {
        "train_smb_control.exe"
    } else {
        "train_smb_control"
    };
    dir.join(exe_name)
}

#[test]
fn train_smb_control_with_help_flag_prints_usage_and_succeeds() {
    for flag in ["--help", "-h"] {
        let output = Command::new(train_smb_control_bin())
            .arg(flag)
            .output()
            .expect("run train_smb_control");

        assert!(output.status.success(), "failed on flag {flag}");
        let stdout = String::from_utf8(output.stdout).expect("stdout utf8");
        assert!(stdout.contains(
            "Usage: train_smb_control <profile_toml> [episodes] [checkpoint_dir] [artifact_dir]"
        ));
    }
}

#[test]
fn train_smb_control_without_required_arguments_prints_usage_and_fails() {
    let output = Command::new(train_smb_control_bin())
        .output()
        .expect("run train_smb_control");

    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).expect("stderr utf8");
    assert!(stderr.contains("Usage:"));
    assert!(
        stderr.contains(
            "train_smb_control <profile_toml> [episodes] [checkpoint_dir] [artifact_dir]"
        )
    );
}
