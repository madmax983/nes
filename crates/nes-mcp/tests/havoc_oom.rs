use std::io::Write;
use std::process::{Command, Stdio};

#[test]
#[ignore = "Havoc OOM Attack"]
fn nes_mcp_stdio_vulnerable_to_oom() {
    let mut child = Command::new(env!("CARGO_BIN_EXE_nes-mcp"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("Failed to spawn nes-mcp");

    let mut stdin = child.stdin.take().unwrap();

    // We send an infinite sequence of bytes without newlines.
    let buf = vec![b'A'; 1024 * 1024]; // 1MB
    for _ in 0..1024 {
        // Try to send 1GB
        if stdin.write_all(&buf).is_err() {
            break;
        }
    }

    let _ = child.kill();
    let _ = child.wait();
}
