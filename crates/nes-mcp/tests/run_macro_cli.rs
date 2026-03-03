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
fn run_macro_without_required_arguments_prints_usage_and_fails() {
    let output = Command::new(run_macro_bin())
        .output()
        .expect("run nes-mcp-run-macro");

    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).expect("stderr utf8");
    assert!(stderr.contains("Usage: nes-mcp-run-macro <rom_path> <script_path>"));
}
