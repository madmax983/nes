#![cfg(feature = "mcp-host")]

use std::io::{BufReader, Write};
use std::net::TcpStream;
use std::time::Duration;

use nes_desktop::mcp_host::{McpHost, read_framed_message};

#[test]
#[should_panic(expected = "Second client blocked waiting for first client!")]
fn havoc_mcp_slowloris_dos() {
    let host = McpHost::start("127.0.0.1:0").expect("host should start");
    let bind_addr = host.bind_addr().to_owned();

    // Client 1: The malicious actor, sending an incomplete payload
    let mut stream1 = TcpStream::connect(&bind_addr).expect("client 1 should connect");
    stream1
        .write_all(b"Content-Length: 100\r\n\r\n{")
        .expect("write partial");

    // Give the server thread a moment to start reading from stream1
    std::thread::sleep(Duration::from_millis(50));

    // Client 2: The legitimate user, who should be able to connect and get a response
    // If the server is blocked by stream1, this will time out or fail.
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
        .set_read_timeout(Some(Duration::from_millis(500)))
        .unwrap();
    let mut reader2 = BufReader::new(stream2);

    let response = read_framed_message(&mut reader2);

    match response {
        Ok(_) => panic!("Client 2 surprisingly received a valid response despite slowloris!"),
        Err(e) => {
            // Panic with the error we want `should_panic` to catch
            panic!(
                "Second client blocked waiting for first client! (got: {})",
                e
            );
        }
    }
}
