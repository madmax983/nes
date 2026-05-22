import re
with open("crates/nes-desktop/tests/havoc.rs", "r") as f:
    content = f.read()

reader_code = """
struct PanicOnInfiniteEofReader<R: std::io::Read> {
    inner: R,
    eof_count: usize,
}

impl<R: std::io::Read> PanicOnInfiniteEofReader<R> {
    fn new(inner: R) -> Self {
        Self { inner, eof_count: 0 }
    }
}

impl<R: std::io::Read> std::io::Read for PanicOnInfiniteEofReader<R> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let res = self.inner.read(buf);
        if let Ok(0) = res {
            self.eof_count += 1;
            if self.eof_count > 100 {
                panic!("Caught infinite loop on EOF!");
            }
        } else {
            self.eof_count = 0;
        }
        res
    }
}
"""

content = content.replace("#[cfg(feature = \"mcp-host\")]\n#[test]", reader_code + "\n#[cfg(feature = \"mcp-host\")]\n#[test]")
content = re.sub(r'BufReader::new\(([^)]+)\)', r'BufReader::new(PanicOnInfiniteEofReader::new(\1))', content)

with open("crates/nes-desktop/tests/havoc.rs", "w") as f:
    f.write(content)
