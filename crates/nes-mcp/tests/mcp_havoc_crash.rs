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

#[test]
#[should_panic(expected = "timeout")]
fn havoc_dos_macro_wait_hang() {
    use nes_core::NesCore;
    use nes_mcp::macro_engine::execute_macro_script;
    use std::sync::mpsc;
    use std::thread;
    use std::time::Duration;

    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        let mut core = NesCore::new();
        // The trigger: A wait command with u64::MAX will hang the thread forever.
        let script = "WAIT 18446744073709551615";
        let _ = execute_macro_script(&mut core, script, None);
        tx.send(()).unwrap();
    });

    // Wait for up to 1 second. If it doesn't finish, we've successfully proven the DoS.
    rx.recv_timeout(Duration::from_millis(500))
        .expect("timeout");
}

#[test]
fn havoc_fuzz_base64_decode_panic() {
    let mut child = Command::new(env!("CARGO_BIN_EXE_nes-mcp"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("spawn nes-mcp daemon");

    let payload_body = r#"{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"export_6502_dsl_rom_base64","arguments":{"source": ".bank 1\nRESET:\n  LDA #$22\n  JMP RESET\n.reset RESET\n"}}}"#;
    let payload = format!("Content-Length: {}\r\n\r\n{}", payload_body.len(), payload_body);
    {
        let stdin = child.stdin.as_mut().expect("child stdin");
        stdin
            .write_all(payload.as_bytes())
            .expect("write bad payload");
        stdin.flush().expect("flush stdin");
    }
    drop(child.stdin.take());

    let status = child.wait().expect("wait on daemon");
    assert!(
        status.success(),
        "Daemon crashed"
    );
}
