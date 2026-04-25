use nes_desktop::mcp_host::read_framed_message;
use std::io::Cursor;

#[test]
#[should_panic(expected = "capacity overflow")]
#[ignore = "Havoc OOM Attack"]
fn havoc_mcp_content_length_oom() {
    // Injecting a massive Content-Length to force capacity overflow panic
    let payload = b"Content-Length: 18446744073709551615\r\n\r\n{}";
    let mut cursor = Cursor::new(payload);

    // This will panic on `vec![0_u8; len]`
    let mut line = String::new();
    let _ = read_framed_message(&mut cursor, &mut line);
}
