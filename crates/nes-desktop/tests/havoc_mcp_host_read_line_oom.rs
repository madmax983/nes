use std::io::BufReader;

struct EndlessNoNewline {
    bytes_read: usize,
    limit: usize,
}

impl std::io::Read for EndlessNoNewline {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        if self.bytes_read > self.limit {
            panic!("Simulated OOM: read_line unbounded allocation exceeded memory limit");
        }
        for b in buf.iter_mut() {
            *b = b'A';
        }
        self.bytes_read += buf.len();
        Ok(buf.len())
    }
}

#[test]
#[should_panic(expected = "Simulated OOM")]
#[ignore = "Havoc DoS Attack"]
fn havoc_mcp_host_read_line_oom() {
    let mut reader = BufReader::new(EndlessNoNewline {
        bytes_read: 0,
        limit: 100_000_000, // 100 MB
    });

    // This will keep reading until the Simulated OOM panic is triggered
    let _ = nes_desktop::mcp_host::read_framed_message(&mut reader);
}
