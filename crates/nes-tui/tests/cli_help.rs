use std::process::Command;

#[test]
fn help_flag_prints_usage_and_default_config_path() {
    let output = Command::new(env!("CARGO_BIN_EXE_nes-tui"))
        .arg("--help")
        .output()
        .expect("nes-tui binary should run");
    assert!(
        output.status.success(),
        "help invocation should exit successfully"
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Usage:"));
    assert!(stderr.contains("Default config path:"));
}
