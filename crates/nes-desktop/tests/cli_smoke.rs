use std::process::Command;

fn nes_desktop_bin() -> std::path::PathBuf {
    if let Ok(path) = std::env::var("CARGO_BIN_EXE_nes-desktop") {
        return std::path::PathBuf::from(path);
    }
    let mut dir = std::env::current_exe().expect("should locate test binary");
    dir.pop();
    if dir.ends_with("deps") {
        dir.pop();
    }
    let exe_name = if cfg!(windows) {
        "nes-desktop.exe"
    } else {
        "nes-desktop"
    };
    dir.join(exe_name)
}

#[test]
fn nes_desktop_missing_rom_prints_styled_error() {
    let output = Command::new(nes_desktop_bin())
        .arg("__does_not_exist__.nes")
        .output()
        .expect("run nes-desktop");

    let stderr = String::from_utf8(output.stderr).expect("stderr utf8");
    assert!(stderr.contains("Could not find the ROM file at"));
    assert!(stderr.contains("__does_not_exist__.nes"));
    assert!(stderr.contains("Hint:"));
    assert!(stderr.contains("Check the path or try the bundled homebrew ROM"));
}

#[test]
fn nes_desktop_invalid_rom_permissions_prints_styled_error() {
    let output = Command::new(nes_desktop_bin())
        .arg(".")
        .output()
        .expect("run nes-desktop");

    let stderr = String::from_utf8(output.stderr).expect("stderr utf8");
    assert!(stderr.contains("Failed to read ROM at"));
    assert!(stderr.contains('.'));
}
