use nes_desktop::mcp_host::read_framed_message;
use std::io::Cursor;

#[test]
fn havoc_mcp_content_length_oom() {
    // Injecting a massive Content-Length to force capacity overflow panic
    let payload = b"Content-Length: 18446744073709551615\r\n\r\n{}";
    let mut cursor = Cursor::new(payload);

    let mut buf = Vec::new();
    let result = read_framed_message(&mut cursor, &mut buf);
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
