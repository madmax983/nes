#![cfg(feature = "mcp-host")]

use nes_desktop::mcp_host::read_framed_message;
use std::io::{BufReader, Write};
use std::net::TcpStream;
use std::time::{Duration, Instant};

use nes_desktop::mcp_host::McpHost;

#[test]
fn havoc_mcp_slowloris_dos() {
    let host = McpHost::start("127.0.0.1:0").expect("host should start");
    let bind_addr = host.bind_addr().to_owned();

    // Client 1: The malicious actor, sending an incomplete payload
    let mut stream1 = TcpStream::connect(&bind_addr).expect("client 1 should connect");
    stream1
        .write_all(b"Content-Length: 100\r\n\r\n{")
        .expect("write partial");

    // Client 2: The legitimate user, who should be able to connect and get a response
    // If the server is blocked by stream1, this will time out or fail.
    let mut stream2 = TcpStream::connect(&bind_addr).expect("client 2 should connect");

    // We send a valid request from client 2. It's a notification, so it doesn't need a response,
    // but the server should be able to parse it without blocking on client 1.
    // Wait, let's use tools/list which gets a response.
    let ping_request = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "ping"
    });

    let payload = serde_json::to_vec(&ping_request).expect("serialize payload");
    stream2
        .write_all(format!("Content-Length: {}\r\n\r\n", payload.len()).as_bytes())
        .expect("write header");
    stream2.write_all(&payload).expect("write payload");
    stream2.flush().expect("flush stream");

    stream2
        .set_read_timeout(Some(Duration::from_secs(2)))
        .expect("set read timeout");
    let mut reader2 = BufReader::new(stream2);

    let start = Instant::now();
    let response = read_framed_message(&mut reader2)
        .expect("should read response")
        .expect("response should not be empty");

    let elapsed = start.elapsed();
    assert!(
        elapsed < Duration::from_secs(5),
        "Second client blocked waiting for first client!"
    );

    let value: serde_json::Value = serde_json::from_slice(&response).expect("deserialize response");
    assert_eq!(value["result"], serde_json::json!({}));
}

#[test]
fn havoc_mcp_content_length_oom() {
    use std::io::Cursor;
    let payload = b"Content-Length: 18446744073709551615\r\n\r\n{}";
    let mut cursor = Cursor::new(payload);
    let result = read_framed_message(&mut cursor);
    assert!(result.is_err());
    assert!(
        result
            .expect_err("test setup failed")
            .contains("exceeds maximum allowed size")
    );
}

#[test]
fn havoc_mcp_read_line_oom() {
    use std::io::Cursor;
    let mut payload = b"Content-Length: 2\r\n".to_vec();
    // Simulate an attacker sending a never-ending header line
    payload.resize(payload.len() + 10 * 1024, b'A');

    let mut cursor = Cursor::new(&payload);
    let result = read_framed_message(&mut cursor);
    assert!(
        result.is_err(),
        "Expected an error from an overly long header line"
    );
}
