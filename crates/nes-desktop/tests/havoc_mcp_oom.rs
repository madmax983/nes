#![cfg(feature = "mcp-host")]

use nes_desktop::mcp_host::read_framed_message;
use std::io::Cursor;

#[test]
fn havoc_mcp_content_length_oom() {
    let payload = b"Content-Length: 18446744073709551615\r\n\r\n{}";
    let mut cursor = Cursor::new(payload);
    let _ = read_framed_message(&mut cursor);
}
