#![cfg(feature = "mcp-host")]

use nes_desktop::mcp_host::read_framed_message;
use std::io::{self, BufRead, Read};

struct OomReader {
    bytes_read: usize,
    limit: usize,
}

impl Read for OomReader {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        if self.bytes_read >= self.limit {
            panic!(
                "OOM simulated: read_line buffered {} bytes without a newline",
                self.bytes_read
            );
        }
        for b in buf.iter_mut() {
            *b = b'A'; // no newline
        }
        self.bytes_read += buf.len();
        Ok(buf.len())
    }
}

// We implement BufRead to satisfy read_framed_message bound.
impl BufRead for OomReader {
    fn fill_buf(&mut self) -> io::Result<&[u8]> {
        unimplemented!()
    }
    fn consume(&mut self, _amt: usize) {
        unimplemented!()
    }
}

#[test]
#[should_panic(expected = "OOM simulated")]
fn read_framed_message_vulnerable_to_oom() {
    let mut reader = io::BufReader::new(OomReader {
        bytes_read: 0,
        limit: 10 * 1024 * 1024,
    });
    // Let's actually test the application code.
    let _ = read_framed_message(&mut reader);
}
