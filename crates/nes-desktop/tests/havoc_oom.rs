#[cfg(feature = "mcp-host")]
#[test]
fn havoc_mcp_host_oom() {
    use nes_desktop::mcp_host::read_framed_message;
    use std::io::BufReader;

    // usize::MAX string representation
    let input = b"Content-Length: 18446744073709551615\r\n\r\n";
    let mut reader = BufReader::new(&input[..]);
    let res = read_framed_message(&mut reader);
    assert!(res.is_err());
}
