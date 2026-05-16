use std::io::{BufReader, Write};
use std::net::TcpStream;
use std::time::{Duration, Instant};

use nes_desktop::mcp_host::{McpHost, read_framed_message};

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

    let payload = serde_json::to_vec(&ping_request).unwrap();
    stream2
        .write_all(format!("Content-Length: {}\r\n\r\n", payload.len()).as_bytes())
        .unwrap();
    stream2.write_all(&payload).unwrap();
    stream2.flush().unwrap();

    stream2
        .set_read_timeout(Some(Duration::from_secs(2)))
        .unwrap();
    let mut reader2 = BufReader::new(stream2);

    let start = Instant::now();
    let mut line = String::new();
    let response = read_framed_message(&mut reader2, &mut line)
        .expect("should read response")
        .expect("response should not be empty");

    let elapsed = start.elapsed();
    assert!(
        elapsed < Duration::from_secs(5),
        "Second client blocked waiting for first client!"
    );

    let value: serde_json::Value = serde_json::from_slice(&response).unwrap();
    assert_eq!(value["result"], serde_json::json!({}));
}
