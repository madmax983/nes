use std::process::Command;

#[test]
fn run_macro_without_required_arguments_prints_usage_and_fails() {
    let binary_path = std::env::var("CARGO_BIN_EXE_run_macro")
        .or_else(|_| std::env::var("CARGO_BIN_EXE_nes-mcp-run-macro"))
        .unwrap_or_else(|_| {
            let exe = if cfg!(windows) {
                "run_macro.exe"
            } else {
                "run_macro"
            };
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("..")
                .join("..")
                .join("target")
                .join("debug")
                .join(exe)
                .to_string_lossy()
                .to_string()
        });
    let output = Command::new(binary_path)
        .output()
        .expect("run nes-mcp-run-macro");

    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).expect("stderr utf8");
    assert!(stderr.contains("Usage: nes-mcp-run-macro <rom_path> <script_path>"));
}
