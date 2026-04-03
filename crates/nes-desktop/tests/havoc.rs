use std::process::Command;

#[test]
#[ignore = "Havoc Logic Flaw Attack"]
fn havoc_crash_nes_desktop_returns_0_on_invalid_args() {
    let mut child = Command::new(env!("CARGO_BIN_EXE_nes-desktop"))
        .arg("--invalid-flag-that-does-not-exist")
        .spawn()
        .expect("spawn nes-desktop");

    let status = child.wait().unwrap();
    // The program exited with 0 (success) despite an invalid flag!
    // This breaks standard POSIX expectations and could break scripts relying on it.
    assert!(status.success(), "It shouldn't succeed, but it does, representing a logic flaw.");
}
