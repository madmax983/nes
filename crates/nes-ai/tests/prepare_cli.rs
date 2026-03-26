use std::process::Command;

fn prepare_smb_control_bin() -> std::path::PathBuf {
    if let Ok(path) = std::env::var("CARGO_BIN_EXE_prepare_smb_control") {
        return std::path::PathBuf::from(path);
    }
    let mut dir = std::env::current_exe().expect("should locate test binary");
    dir.pop();
    if dir.ends_with("deps") {
        dir.pop();
    }
    let exe_name = if cfg!(windows) {
        "prepare_smb_control.exe"
    } else {
        "prepare_smb_control"
    };
    dir.join(exe_name)
}

#[test]
fn prepare_smb_control_missing_rom_prints_styled_error() {
    let output = Command::new(prepare_smb_control_bin())
        .arg("__does_not_exist__.nes")
        .arg("movie.json")
        .arg("out.bin")
        .output()
        .expect("run prepare_smb_control");

    let stderr = String::from_utf8(output.stderr).expect("stderr utf8");
    assert!(stderr.contains("Could not find the ROM file at"));
    assert!(stderr.contains("__does_not_exist__.nes"));
    assert!(stderr.contains("Hint:"));
    assert!(stderr.contains("Check the path or try the bundled homebrew ROM"));
}

#[test]
fn prepare_smb_control_missing_movie_json_prints_error() {
    let output = Command::new(prepare_smb_control_bin())
        .arg("roms/homebrew/homebrew.nes")
        .arg("__does_not_exist__.json")
        .arg("out.bin")
        .output()
        .expect("run prepare_smb_control");

    assert!(!output.status.success());
}

#[test]
fn prepare_smb_control_invalid_rom_permissions_prints_styled_error() {
    let output = Command::new(prepare_smb_control_bin())
        .arg(".")
        .arg("movie.json")
        .arg("out.bin")
        .output()
        .expect("run prepare_smb_control");

    let stderr = String::from_utf8(output.stderr).expect("stderr utf8");
    assert!(stderr.contains("Failed to read ROM at"));
    assert!(stderr.contains("'.'"));
}

#[test]
fn prepare_smb_control_with_no_args_prints_usage_and_fails() {
    let output = Command::new(prepare_smb_control_bin())
        .output()
        .expect("run prepare_smb_control");

    assert!(!output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout utf8");
    assert!(
        stdout.contains(
            "Usage: prepare_smb_control <rom_path> <bootstrap_tas_json> <output_snapshot>"
        )
    );
}

#[test]
fn prepare_smb_control_with_help_flag_prints_usage_and_succeeds() {
    for flag in ["--help", "-h"] {
        let output = Command::new(prepare_smb_control_bin())
            .arg(flag)
            .output()
            .expect("run prepare_smb_control");

        assert!(output.status.success(), "failed on flag {flag}");
        let stdout = String::from_utf8(output.stdout).expect("stdout utf8");
        assert!(stdout.contains(
            "Usage: prepare_smb_control <rom_path> <bootstrap_tas_json> <output_snapshot>"
        ));
    }
}
