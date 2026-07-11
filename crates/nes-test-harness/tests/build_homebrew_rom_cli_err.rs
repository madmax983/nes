use std::process::Command;

#[test]
fn build_homebrew_rom_fails_if_run_returns_err() {
    let output = Command::new("cargo")
        .args(&["run", "--bin", "build_homebrew_rom", "--", "--out", "/does/not/exist/12345/dir/that/fails/to/create"])
        .output()
        .expect("Failed to execute process");

    // it should exit with 1
    assert!(!output.status.success());
    assert_eq!(output.status.code(), Some(1));
}
