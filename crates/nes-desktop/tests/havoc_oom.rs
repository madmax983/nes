#[cfg(feature = "mcp-host")]
#[test]
#[should_panic(expected = "capacity overflow")]
fn havoc_mcp_host_oom() {
    use nes_desktop::mcp_host::read_framed_message;
    use std::io::BufReader;

    // usize::MAX string representation
    let input = b"Content-Length: 18446744073709551615\r\n\r\n";
    let mut reader = BufReader::new(&input[..]);
    let mut line = String::new();
    let _ = read_framed_message(&mut reader, &mut line);
}
