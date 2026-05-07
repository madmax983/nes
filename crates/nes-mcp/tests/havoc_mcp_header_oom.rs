use std::io::Write;
use std::process::{Command, Stdio};

#[test]
#[ignore = "Havoc OOM Attack (SIGKILL)"]
fn havoc_crash_mcp_daemon_header_oom() {
    let mut child = Command::new(env!("CARGO_BIN_EXE_nes-mcp"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("spawn nes-mcp daemon");

    // The Kill Switch: Send an infinitely long header line without a newline.
    // The server's `read_line` will buffer it in memory until the process is OOM-killed.
    let mut stdin = child.stdin.take().expect("child stdin");

    let chunk = vec![b' '; 1024 * 1024 * 10]; // 10MB chunk

    let _t = std::thread::spawn(move || {
        loop {
            if stdin.write_all(&chunk).is_err() {
                break; // Broken pipe when the child dies
            }
        }
    });

    let status = child.wait().expect("wait on daemon");

    assert!(
        !status.success(),
        "Daemon survived the OOM attack. This should not happen."
    );
}
