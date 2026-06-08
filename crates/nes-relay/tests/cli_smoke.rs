use std::process::Command;

#[test]
fn help_flag_prints_usage_to_stderr_and_exits_nonzero() {
    let output = Command::new(env!("CARGO_BIN_EXE_nes-relay"))
        .arg("--help")
        .output()
        .expect("run nes-relay --help");

    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).expect("stderr utf8");
    assert!(stderr.contains("nes-relay [--bind <addr>]"));
}
