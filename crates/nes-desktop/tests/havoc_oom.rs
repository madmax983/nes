#[cfg(feature = "mcp-host")]
#[test]
fn havoc_mcp_host_oom() {
    use nes_desktop::mcp_host::read_framed_message;
    use std::io::BufReader;

    // usize::MAX string representation
    let input = b"Content-Length: 18446744073709551615\r\n\r\n";
    let mut reader = BufReader::new(&input[..]);
    let result = read_framed_message(&mut reader);

    assert!(result.is_err(), "Expected reading an oversized content-length to result in an error");
    assert!(
        result.unwrap_err().contains("exceeds maximum allowed size"),
        "Error message did not contain expected capacity limit text."
    );
}
