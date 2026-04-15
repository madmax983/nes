use std::process::Command;

#[test]
fn help_flag_prints_usage_to_stdout_and_exits_zero() {
    let output = Command::new(env!("CARGO_BIN_EXE_nes-relay"))
        .arg("--help")
        .output()
        .expect("run nes-relay --help");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout utf8");
    assert!(stdout.contains("Usage: nes-relay"));
}
