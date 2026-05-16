#![cfg(feature = "mcp-host")]

use nes_desktop::mcp_host::read_framed_message;
use std::io::{Cursor, Read};

#[test]
fn havoc_mcp_content_length_oom() {
    let payload = b"Content-Length: 18446744073709551615\r\n\r\n{}";
    let mut cursor = Cursor::new(payload);

    let result = read_framed_message(&mut cursor);
    assert!(result.is_err());
    let err_msg = result.unwrap_err();
    assert!(err_msg.contains("exceeds maximum allowed size"));
}

struct PanicOnEofReader<R: Read> {
    inner: R,
    zero_reads: usize,
}

impl<R: Read> PanicOnEofReader<R> {
    fn new(inner: R) -> Self {
        Self {
            inner,
            zero_reads: 0,
        }
    }
}

impl<R: Read + std::io::BufRead> std::io::BufRead for PanicOnEofReader<R> {
    fn fill_buf(&mut self) -> std::io::Result<&[u8]> {
        let buf = self.inner.fill_buf()?;
        if buf.is_empty() {
            self.zero_reads += 1;
            if self.zero_reads > 10 {
                panic!("Infinite loop detected: read 0 bytes too many times!");
            }
        } else {
            self.zero_reads = 0;
        }
        Ok(buf)
    }

    fn consume(&mut self, amt: usize) {
        self.inner.consume(amt);
    }
}

impl<R: Read> Read for PanicOnEofReader<R> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let n = self.inner.read(buf)?;
        if n == 0 {
            self.zero_reads += 1;
            if self.zero_reads > 10 {
                panic!("Infinite loop detected: read 0 bytes too many times!");
            }
        } else {
            self.zero_reads = 0;
        }
        Ok(n)
    }
}

#[test]
fn havoc_mcp_host_read_framed_message_invalid_loop() {
    let payload = b"Content-Type: application/vscode-jsonrpc; charset=utf-8\r\n\r\n";
    let cursor = Cursor::new(payload);
    let mut reader = PanicOnEofReader::new(std::io::BufReader::new(cursor));

    let result = read_framed_message(&mut reader);
    assert!(result.is_err(), "should err ");
}
