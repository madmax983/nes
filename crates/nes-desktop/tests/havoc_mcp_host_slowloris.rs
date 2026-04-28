#[cfg(feature = "mcp-host")]
#[test]
#[should_panic(expected = "timeout")]
#[ignore = "Havoc DoS Attack"]
fn havoc_mcp_host_slowloris() {
    use nes_desktop::mcp_host::McpHost;
    use std::io::{Read, Write};
    use std::net::TcpStream;
    use std::sync::mpsc;
    use std::thread;
    use std::time::Duration;

    // Start the host on a random loopback port
    let host = McpHost::start("127.0.0.1:0").expect("start mcp host");
    let addr = host.bind_addr().to_owned();

    // Attack: First client connects and sends incomplete data, holding the connection open.
    // Because `mcp_host` handles clients synchronously without spawned threads or timeouts,
    // this single connection completely blocks the server from accepting or processing any other clients.
    let mut attacker = TcpStream::connect(&addr).expect("attacker connects");
    attacker
        .write_all(b"Content-Length: 100\r\n\r\n")
        .expect("send partial headers");

    // Sleep briefly to ensure the attacker is being handled
    thread::sleep(Duration::from_millis(100));

    // Victim: Second client attempts to connect and communicate
    let (victim_tx, victim_rx) = mpsc::channel();
    thread::spawn(move || {
        let mut victim = TcpStream::connect(&addr).expect("victim connects");

        // This request will never be answered because the host thread is hung reading from the attacker
        let req = r#"{"jsonrpc": "2.0", "method": "tools/list", "id": 1}"#;
        let payload = format!("Content-Length: {}\r\n\r\n{}", req.len(), req);
        victim
            .write_all(payload.as_bytes())
            .expect("victim sends request");

        let mut buf = [0u8; 1024];
        let n = victim.read(&mut buf).expect("victim read");
        victim_tx.send(n).unwrap();
    });

    // We wait to see if the victim ever gets a response. If it doesn't, the server is successfully deadlocked.
    victim_rx
        .recv_timeout(Duration::from_millis(500))
        .expect("timeout");
}
