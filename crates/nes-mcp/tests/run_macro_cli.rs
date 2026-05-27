use std::process::Command;

/// Resolve the path to the `run_macro` binary.
///
/// `CARGO_BIN_EXE_run_macro` is set by `cargo test` but not always by
/// `cargo llvm-cov` (which uses a non-standard target directory).  As a
/// reliable fallback we locate the test binary itself — it lives in the same
/// target directory (or in `deps/` below it) as all other compiled artifacts
/// for this build, including `run_macro`.
fn run_macro_bin() -> std::path::PathBuf {
    if let Ok(path) = std::env::var("CARGO_BIN_EXE_run_macro") {
        return std::path::PathBuf::from(path);
    }

    // Walk up from the test binary's own location to find the target dir.
    let mut dir = std::env::current_exe().expect("should locate test binary");
    dir.pop(); // remove test binary filename
    if dir.ends_with("deps") {
        dir.pop(); // target/<profile>/deps -> target/<profile>
    }

    let exe_name = if cfg!(windows) {
        "run_macro.exe"
    } else {
        "run_macro"
    };
    dir.join(exe_name)
}

#[test]
fn run_macro_with_missing_rom_prints_styled_error() {
    let output = Command::new(run_macro_bin())
        .arg("__does_not_exist__.nes")
        .arg("dummy.txt")
        .output()
        .expect("run nes-mcp-run-macro");

    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).expect("stderr utf8");
    assert!(stderr.contains("Could not find the ROM file"));
    assert!(stderr.contains("__does_not_exist__.nes"));
    assert!(stderr.contains("Hint"));
    assert!(stderr.contains("Check the path or try the bundled homebrew ROM"));
}

#[test]
fn run_macro_with_missing_script_prints_styled_error() {
    let temp_dir = std::env::temp_dir().join("nes_mcp_run_macro_missing_script_cli_test");
    let _ = std::fs::remove_dir_all(&temp_dir);
    std::fs::create_dir_all(&temp_dir).unwrap();
    let rom_path = create_dummy_rom_file(&temp_dir);

    let output = Command::new(run_macro_bin())
        .arg(&rom_path)
        .arg("__does_not_exist__.txt")
        .output()
        .expect("run nes-mcp-run-macro");

    let _ = std::fs::remove_dir_all(&temp_dir);

    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).expect("stderr utf8");
    assert!(stderr.contains("Could not find the macro script at"));
    assert!(stderr.contains("__does_not_exist__.txt"));
    assert!(stderr.contains("Hint"));
    assert!(stderr.contains("Check the path or create a new .txt file"));
}

#[test]
fn run_macro_with_invalid_rom_permissions_prints_styled_error() {
    let output = Command::new(run_macro_bin())
        .arg(".")
        .arg("dummy.txt")
        .output()
        .expect("run nes-mcp-run-macro");

    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).expect("stderr utf8");
    assert!(stderr.contains("Failed to read ROM"));
    assert!(stderr.contains('.'));
}

#[test]
fn run_macro_without_required_arguments_prints_usage_and_fails() {
    let output = Command::new(run_macro_bin())
        .output()
        .expect("run nes-mcp-run-macro");

    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).expect("stderr utf8");
    assert!(stderr.contains("Usage: nes-mcp-run-macro <rom_path> <script_path>"));
}

fn create_dummy_rom_file(dir: &std::path::Path) -> std::path::PathBuf {
    let path = dir.join("dummy.nes");
    // Minimal valid iNES header with 1 PRG bank (16 bytes header + 16384 bytes PRG)
    let mut rom_bytes = vec![0; 16 + 16384];
    rom_bytes[0..4].copy_from_slice(b"NES\x1A"); // Magic string
    rom_bytes[4] = 1; // PRG ROM Size (16KB)
    rom_bytes[5] = 0; // CHR ROM Size (0KB - uses RAM)
    std::fs::write(&path, &rom_bytes).unwrap();
    path
}

#[test]
fn run_macro_with_valid_script_prints_progress() {
    let temp_dir = std::env::temp_dir().join("nes_mcp_run_macro_test");
    let _ = std::fs::remove_dir_all(&temp_dir);
    std::fs::create_dir_all(&temp_dir).unwrap();

    let rom_path = create_dummy_rom_file(&temp_dir);

    let script_path = temp_dir.join("script.txt");
    std::fs::write(&script_path, "WAIT 1\nPRESS A\nRELEASE A\n").unwrap();

    let output = Command::new(run_macro_bin())
        .arg(&rom_path)
        .arg(&script_path)
        .output()
        .expect("run nes-mcp-run-macro");

    let _ = std::fs::remove_dir_all(&temp_dir);

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout utf8");
    assert!(stdout.contains("Executing Macro Script..."));
    // The progress lines should be printed
    assert!(stdout.contains("Running line 1 / 3"));
    assert!(stdout.contains("Running line 2 / 3"));
    assert!(stdout.contains("Running line 3 / 3"));
    assert!(stdout.contains("Execution Complete"));
}

#[test]
fn run_macro_with_empty_script_prints_zero_progress() {
    let temp_dir = std::env::temp_dir().join("nes_mcp_run_macro_empty_test");
    let _ = std::fs::remove_dir_all(&temp_dir);
    std::fs::create_dir_all(&temp_dir).unwrap();

    let rom_path = create_dummy_rom_file(&temp_dir);

    let script_path = temp_dir.join("empty.txt");
    std::fs::write(&script_path, "").unwrap();

    let output = Command::new(run_macro_bin())
        .arg(&rom_path)
        .arg(&script_path)
        .output()
        .expect("run nes-mcp-run-macro");

    let _ = std::fs::remove_dir_all(&temp_dir);

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout utf8");
    assert!(stdout.contains("Executing Macro Script..."));
    assert!(stdout.contains("Execution Complete"));
}

#[test]
fn run_macro_with_help_flag_prints_usage_and_succeeds() {
    for flag in ["--help", "-h"] {
        let output = Command::new(run_macro_bin())
            .arg(flag)
            .output()
            .expect("run nes-mcp-run-macro");

        assert!(output.status.success());
        let stdout = String::from_utf8(output.stdout).expect("stdout utf8");
        assert!(stdout.contains("Usage: nes-mcp-run-macro <rom_path> <script_path>"));
    }
}
