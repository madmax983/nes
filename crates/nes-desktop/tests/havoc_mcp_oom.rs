#![cfg(feature = "mcp-host")]

use nes_desktop::mcp_host::read_framed_message;
use std::io::{BufReader, Cursor};

#[test]
fn havoc_mcp_oom_infinite_headers() {
    let payload = vec![b'A'; 10000]; // 10000 bytes without newline
    let mut reader = BufReader::new(Cursor::new(payload));
    let err = read_framed_message(&mut reader).unwrap_err();
    assert!(err.contains("MCP headers exceeded maximum length"));
}

#[test]
fn havoc_mcp_exact_max_headers_length() {
    let payload = vec![b'A'; 8192];
    let mut reader = BufReader::new(Cursor::new(payload));
    let result = read_framed_message(&mut reader);
    assert!(result.unwrap().is_none());
}
