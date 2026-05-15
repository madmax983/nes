#[cfg(feature = "mcp-host")]
use nes_desktop::mcp_host::read_framed_message;

#[cfg(feature = "mcp-host")]
#[test]
#[should_panic(expected = "capacity overflow")]
#[ignore = "havoc target"]
fn havoc_mcp_host_oom_on_massive_content_length() {
    use std::io::{BufReader, Cursor};
    // The vulnerability is in `read_framed_message` inside `mcp_host.rs`.
    // It allocates `vec![0_u8; len]` based solely on the Content-Length header, without limits.
    let mut reader = BufReader::new(Cursor::new(
        b"Content-Length: 18446744073709551615\r\n\r\n".to_vec(),
    ));
    let _ = read_framed_message(&mut reader);
}
