use std::process::Command;

fn eval_smb_control_bin() -> std::path::PathBuf {
    if let Ok(path) = std::env::var("CARGO_BIN_EXE_eval_smb_control") {
        return std::path::PathBuf::from(path);
    }
    let mut dir = std::env::current_exe().expect("should locate test binary");
    dir.pop();
    if dir.ends_with("deps") {
        dir.pop();
    }
    let exe_name = if cfg!(windows) {
        "eval_smb_control.exe"
    } else {
        "eval_smb_control"
    };
    dir.join(exe_name)
}

#[test]
fn eval_smb_control_with_no_args_prints_usage_and_fails() {
    let output = Command::new(eval_smb_control_bin())
        .output()
        .expect("run eval_smb_control");

    assert!(!output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout utf8");
    assert!(stdout.contains(
        "Usage: eval_smb_control <profile_toml> <checkpoint_base> [episodes] [artifact_dir]"
    ));
}

#[test]
fn eval_smb_control_with_help_flag_prints_usage_and_succeeds() {
    for flag in ["--help", "-h"] {
        let output = Command::new(eval_smb_control_bin())
            .arg(flag)
            .output()
            .expect("run eval_smb_control");

        assert!(output.status.success(), "failed on flag {flag}");
        let stdout = String::from_utf8(output.stdout).expect("stdout utf8");
        assert!(stdout.contains(
            "Usage: eval_smb_control <profile_toml> <checkpoint_base> [episodes] [artifact_dir]"
        ));
    }
}
