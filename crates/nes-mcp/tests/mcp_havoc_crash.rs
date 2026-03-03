use std::io::Write;
use std::process::{Command, Stdio};

#[test]
fn havoc_crash_mcp_daemon_oom() {
    let mut child = Command::new(env!("CARGO_BIN_EXE_nes-mcp"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("spawn nes-mcp daemon");

    // The Kill Switch: The payload tells the MCP daemon to allocate
    // 18.44 Exabytes of memory for a JSON-RPC request.
    let payload = "Content-Length: 18446744073709551615\r\n\r\n";
    {
        let stdin = child.stdin.as_mut().expect("child stdin");
        stdin
            .write_all(payload.as_bytes())
            .expect("write bad payload");
        stdin.flush().expect("flush stdin");
    }
    drop(child.stdin.take());

    let status = child.wait().expect("wait on daemon");

    // The daemon should crash from capacity overflow, leading to an aborted/failed exit status.
    // If it survives (status.success() == true), then Havoc failed.
    assert!(
        !status.success(),
        "Daemon survived the OOM attack. This should not happen."
    );
}
