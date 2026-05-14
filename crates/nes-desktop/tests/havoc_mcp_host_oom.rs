#![cfg(feature = "mcp-host")]

use nes_desktop::mcp_host::read_framed_message;
use std::io::{BufReader, Read};

struct EndlessNoNewlineReader {
    bytes_read: usize,
    limit: usize,
}

impl Read for EndlessNoNewlineReader {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        for item in buf.iter_mut() {
            *item = b'A';
        }
        self.bytes_read += buf.len();
        if self.bytes_read > self.limit {
            panic!(
                "OOM vulnerability triggered! read_line consumed too much memory without a newline."
            );
        }
        Ok(buf.len())
    }
}

#[test]
#[should_panic]
#[ignore = "Havoc OOM Attack"]
fn havoc_test_mcp_host_read_framed_message_oom() {
    let mut reader = BufReader::new(EndlessNoNewlineReader {
        bytes_read: 0,
        limit: 10 * 1024 * 1024,
    });
    let _ = read_framed_message(&mut reader);
}
