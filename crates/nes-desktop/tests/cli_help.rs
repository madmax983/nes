use std::process::Command;

#[test]
fn help_flag_prints_usage_and_default_config_path() {
    let output = Command::new(env!("CARGO_BIN_EXE_nes-desktop"))
        .arg("--help")
        .output()
        .expect("nes-desktop binary should run");
    assert!(
        output.status.success(),
        "help invocation should exit successfully"
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Usage: nes-desktop"));
    assert!(stdout.contains("Default config path:"));
    assert!(stdout.contains("--cheat-code <code>"));
}
