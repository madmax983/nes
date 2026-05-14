#![cfg(feature = "mcp-host")]

use nes_desktop::mcp_host::read_framed_message;
use std::io::{BufReader, Cursor};

#[test]
fn havoc_mcp_content_length_oom() {
    // Injecting a massive Content-Length to force capacity overflow panic
    let payload = b"Content-Length: 18446744073709551615\r\n\r\n{}";
    let mut cursor = BufReader::new(Cursor::new(payload));

    let result = read_framed_message(&mut cursor);
    assert!(
        result.is_err(),
        "Expected reading an oversized content-length to result in an error"
    );
    let err_msg = result.unwrap_err();
    assert!(
        err_msg.contains("exceeds maximum allowed size"),
        "Error message did not contain expected text. Got: {}",
        err_msg
    );
}

#[test]
fn havoc_mcp_missing_newline_oom() {
    let payload = b"Content-Length: 10\r\n".to_vec();
    let mut bad_header = payload.clone();
    bad_header.extend(vec![b'A'; 10000]); // 10KB of header with no newline

    let mut cursor = BufReader::new(Cursor::new(bad_header));
    let result = read_framed_message(&mut cursor);
    assert!(
        result.is_err(),
        "Expected oversized header line without newline to fail"
    );
    let err_msg = result.unwrap_err();
    assert!(
        err_msg.contains("exceeds maximum"),
        "Error message did not contain expected text. Got: {}",
        err_msg
    );
}
