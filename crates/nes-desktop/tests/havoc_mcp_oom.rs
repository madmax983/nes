#![cfg(feature = "mcp-host")]

use nes_desktop::mcp_host::read_framed_message;
use std::io::Cursor;

#[test]
fn havoc_mcp_content_length_oom() {
    let payload = b"Content-Length: 18446744073709551615\r\n\r\n{}";
    let mut cursor = Cursor::new(payload);

    let result = read_framed_message(&mut cursor);
    assert!(result.is_err());
    let err_msg = result.unwrap_err();
    assert!(err_msg.contains("exceeds maximum allowed size"));
}
