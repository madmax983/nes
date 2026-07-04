#![cfg(feature = "mcp-host")]
use std::io::Write;
use std::net::TcpStream;
use std::time::{Duration, Instant};

use nes_core::NesCore;
use nes_desktop::mcp_host::McpHost;

#[test]
fn havoc_mcp_slowloris_dos() {
    let host = McpHost::start("127.0.0.1:0").expect("Failed to start host");
    let bind_addr = host.bind_addr();

    let mut core = NesCore::new();
    let mut streams = vec![];
    for _ in 0..10 {
        if let Ok(mut stream) = TcpStream::connect(bind_addr) {
            stream.write_all(b"Content-Length: 100\r\n").unwrap();
            streams.push(stream);
        }
    }

    let start = Instant::now();
    while start.elapsed() < Duration::from_millis(100) {
        host.drain(&mut core);
        std::thread::sleep(Duration::from_millis(10));
    }
}
