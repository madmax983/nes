struct InfiniteNoNewlineReader {
    bytes_read: usize,
    limit: usize,
}
impl std::io::Read for InfiniteNoNewlineReader {
    fn read(&mut self, _buf: &mut [u8]) -> std::io::Result<usize> {
        unimplemented!()
    }
}
impl std::io::BufRead for InfiniteNoNewlineReader {
    fn fill_buf(&mut self) -> std::io::Result<&[u8]> {
        if self.bytes_read > self.limit {
            panic!("Simulated OOM: read_line consumed too much memory without a newline");
        }
        static BUF: [u8; 1024] = [b'x'; 1024];
        Ok(&BUF)
    }
    fn consume(&mut self, amt: usize) {
        self.bytes_read += amt;
    }
}

fn main() {
    let mut reader = InfiniteNoNewlineReader { bytes_read: 0, limit: 1_000_000 };
    let mut line = String::new();
    let _ = std::io::BufRead::read_line(&mut reader, &mut line).unwrap();
}
