#![cfg(feature = "mcp-host")]

use nes_desktop::mcp_host::read_framed_message;
use std::io::Cursor;

#[test]
#[ignore = "Havoc OOM Attack (SIGKILL)"]
fn havoc_mcp_host_oom_on_massive_content_length() {
    let payload = b"Content-Length: 18446744073709551615\r\n\r\n{}";
    let mut reader = Cursor::new(payload);

    let result = read_framed_message(&mut reader);

    assert!(
        result.is_err(),
        "Expected OOM to kill process or read failure"
    );
}
