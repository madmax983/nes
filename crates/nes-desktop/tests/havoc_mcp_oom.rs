#[cfg(feature = "mcp-host")]
use nes_desktop::mcp_host::read_framed_message;
#[cfg(feature = "mcp-host")]
use std::io::{BufRead, Read};

#[cfg(feature = "mcp-host")]
struct OomReader {
    bytes_read: usize,
}

#[cfg(feature = "mcp-host")]
impl Read for OomReader {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let n = buf.len();
        for item in buf.iter_mut().take(n) {
            *item = b'a';
        }
        self.bytes_read += n;
        if self.bytes_read > 10 * 1024 * 1024 {
            panic!("Havoc OOM trigger: read_line consumed more than 10MB without a newline!");
        }
        Ok(n)
    }
}

#[cfg(feature = "mcp-host")]
impl BufRead for OomReader {
    fn fill_buf(&mut self) -> std::io::Result<&[u8]> {
        unimplemented!()
    }
    fn consume(&mut self, _amt: usize) {}
}

#[cfg(feature = "mcp-host")]
#[test]
#[should_panic(
    expected = "Havoc OOM trigger: read_line consumed more than 10MB without a newline!"
)]
#[ignore = "havoc target"]
fn havoc_test_read_line_oom() {
    let mut reader = std::io::BufReader::new(OomReader { bytes_read: 0 });
    let _ = read_framed_message(&mut reader);
}
