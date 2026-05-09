use std::io::Write;
use std::process::{Command, Stdio};
use std::thread;

#[test]
#[ignore]
fn havoc_read_line_oom() {
    let mut child = Command::new(env!("CARGO_BIN_EXE_nes-mcp"))
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("Failed to start nes-mcp");

    let mut stdin = child.stdin.take().expect("Failed to open stdin");

    let handle = thread::spawn(move || {
        let chunk = vec![b'A'; 1024 * 1024];
        loop {
            if stdin.write_all(&chunk).is_err() {
                break;
            }
        }
    });

    let status = child.wait().expect("Failed to wait on child");
    handle.join().unwrap();

    assert!(!status.success(), "Process exited normally instead of crashing due to capacity overflow/OOM");
}
