use std::process::{Command, Stdio};
use std::io::Write;
use std::thread;

#[test]
#[ignore]
fn test_mcp_read_line_oom() {
    let mut child = Command::new(env!("CARGO_BIN_EXE_nes-mcp"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("Failed to start nes-mcp");

    let mut stdin = child.stdin.take().expect("Failed to open stdin");

    // Continuously write data without newlines to cause read_line to buffer indefinitely
    let handle = thread::spawn(move || {
        let chunk = vec![b'A'; 1024 * 1024]; // 1MB chunks
        loop {
            if stdin.write_all(&chunk).is_err() {
                break;
            }
        }
    });

    // Wait for the child to die from OOM (SIGKILL)
    let status = child.wait().expect("Failed to wait on child");
    assert!(!status.success());

    let _ = child.kill();
}
