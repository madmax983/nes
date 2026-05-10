#[cfg(feature = "mcp-host")]
use std::io::{BufReader, Write};
#[cfg(feature = "mcp-host")]
use std::net::TcpStream;
#[cfg(feature = "mcp-host")]
use std::time::{Duration, Instant};

#[cfg(feature = "mcp-host")]
use nes_desktop::mcp_host::{McpHost, read_framed_message};

#[cfg(feature = "mcp-host")]
#[test]
fn havoc_mcp_slowloris_dos() {
    let host = McpHost::start("127.0.0.1:0").expect("host should start");
    let bind_addr = host.bind_addr().to_owned();

    let mut stream1 = TcpStream::connect(&bind_addr).expect("client 1 should connect");
    stream1
        .write_all(b"Content-Length: 100\r\n\r\n{")
        .expect("write partial");

    let mut stream2 = TcpStream::connect(&bind_addr).expect("client 2 should connect");

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
    let response = read_framed_message(&mut reader2)
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

#[cfg(feature = "mcp-host")]
#[test]
fn test_read_framed_message_empty_payload() {
    let data = b"Content-Length: 0\r\n\r\n";
    let mut reader = std::io::BufReader::new(std::io::Cursor::new(data));
    let result = read_framed_message(&mut reader).unwrap();
    assert_eq!(result.unwrap(), b"");
}

#[cfg(feature = "mcp-host")]
#[test]
fn test_read_framed_message_unexpected_eof() {
    let data = b"Content-Length: 10\r\n"; // Incomplete headers
    let mut reader = std::io::BufReader::new(std::io::Cursor::new(data));
    let result = read_framed_message(&mut reader);
    assert_eq!(result, Err("unexpected EOF while reading MCP headers".to_owned()));
}
