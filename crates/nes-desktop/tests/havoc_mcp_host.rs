#![cfg(feature = "mcp-host")]
use std::io::{Read, Write};
use std::net::TcpStream;
use std::time::Duration;

#[test]
#[ignore = "havoc target"]
fn havoc_test_mcp_host_slowloris() {
    let host = nes_desktop::mcp_host::McpHost::start("127.0.0.1:0").expect("host should start");
    let bind_addr = host.bind_addr().to_owned();

    let mut stream = TcpStream::connect(&bind_addr).expect("client connect");

    // Read timeout to prevent hanging forever
    stream
        .set_read_timeout(Some(Duration::from_millis(500)))
        .unwrap();

    let header = format!("Content-Length: {}\r\n\r\n", 9999999);
    stream.write_all(header.as_bytes()).expect("write header");

    let mut stream2 = TcpStream::connect(&bind_addr).expect("client 2 connect");
    stream2
        .set_read_timeout(Some(Duration::from_millis(500)))
        .unwrap();
    stream2
        .write_all(b"Content-Length: 2\r\n\r\n{}")
        .expect("client 2 write");

    let mut buf = [0; 1024];
    let res = stream2.read(&mut buf);

    if res.is_err() && res.unwrap_err().kind() == std::io::ErrorKind::WouldBlock {
        panic!("McpHost is blocked by the first client!");
    }
}
