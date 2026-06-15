use std::process::Command;

#[test]
fn help_flag_prints_usage_and_default_config_path() {
    let output = Command::new(env!("CARGO_BIN_EXE_nes-desktop"))
        .arg("--help")
        .output()
        .expect("nes-desktop binary should run");
    assert!(
        output.status.success() || output.status.code() == Some(0), // It exits with 0 now
        "help invocation should exit successfully"
    );

    let output_str = String::from_utf8_lossy(&output.stdout).into_owned() + &String::from_utf8_lossy(&output.stderr);
    assert!(output_str.contains("Usage: nes-desktop"));
    assert!(output_str.contains("Default config path:"));
    assert!(output_str.contains("--cheat-code <code>"));
}
