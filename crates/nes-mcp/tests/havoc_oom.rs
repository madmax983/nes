use std::process::{Command, Stdio};
use std::io::Write;
use std::thread;

#[test]
#[ignore]
fn test_mcp_oom_vulnerability() {
    let mut child = Command::new(env!("CARGO_BIN_EXE_nes-mcp"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to spawn nes-mcp");

    let mut stdin = child.stdin.take().expect("Failed to open stdin");

    // Continuously write large byte chunks without a newline
    let chunk = vec![b'A'; 1024 * 1024]; // 1MB chunk
    let write_thread = thread::spawn(move || {
        loop {
            if stdin.write_all(&chunk).is_err() {
                break;
            }
        }
    });

    // Wait for process to die
    let status = child.wait().expect("Failed to wait on child");
    write_thread.join().unwrap();

    // The process should be killed by OS (SIGKILL) due to OOM
    // If it gracefully handles it, it would exit with an error. But it won't.
    // In Rust, SIGKILL on Unix means status.success() is false and status.code() is None
    assert!(!status.success());
    assert!(status.code().is_none(), "Expected termination by signal (SIGKILL)");
}
