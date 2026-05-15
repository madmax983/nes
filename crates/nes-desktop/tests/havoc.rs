use nes_desktop::args::parse_runtime_args;
use nes_desktop::session_cheats::SessionCheats;
use proptest::prelude::*;

proptest! {
    #![proptest_config(ProptestConfig::with_cases(5000))]
    #[test]
    #[ignore = "havoc target"]
    fn havoc_fuzz_session_cheats(raw_code in ".*") {
        let mut cheats = SessionCheats::new();
        let _ = cheats.add(&raw_code);
    }

    #[test]
    #[ignore = "havoc target"]
    fn havoc_fuzz_session_cheats_multiple(
        codes in proptest::collection::vec(".*", 0..10),
    ) {
        let _ = SessionCheats::from_raw_codes(&codes);
    }

    #[test]
    #[ignore = "havoc target"]
    fn havoc_fuzz_desktop_args(
        args in proptest::collection::vec(".*", 0..10),
    ) {
        let _ = parse_runtime_args(&args);
    }
}

// In rust, integration tests can access pub modules from the library.
// We exposed mcp_host so we can test the exact vulnerability.
#[cfg(feature = "mcp-host")]
#[test]
#[should_panic(expected = "capacity overflow")]
#[ignore = "havoc target"]
fn havoc_mcp_host_oom_on_massive_content_length() {
    use std::io::{BufReader, Cursor};
    // The vulnerability is in `read_framed_message` inside `mcp_host.rs`.
    // It allocates `vec![0_u8; len]` based solely on the Content-Length header, without limits.
    let mut reader = BufReader::new(Cursor::new(
        b"Content-Length: 18446744073709551615\r\n\r\n".to_vec(),
    ));
    let _ = nes_desktop::mcp_host::read_framed_message(&mut reader);
}

#[cfg(feature = "mcp-host")]
struct InfiniteNoNewlineReader {
    bytes_read: usize,
    limit: usize,
}
#[cfg(feature = "mcp-host")]
impl std::io::Read for InfiniteNoNewlineReader {
    fn read(&mut self, _buf: &mut [u8]) -> std::io::Result<usize> {
        unimplemented!()
    }
}
#[cfg(feature = "mcp-host")]
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

#[cfg(feature = "mcp-host")]
#[test]
#[should_panic(expected = "Simulated OOM")]
#[ignore = "Havoc OOM Attack"]
fn havoc_mcp_host_oom_on_missing_newline() {
    let mut reader = InfiniteNoNewlineReader {
        bytes_read: 0,
        limit: 10_000_000,
    };
    let _ = nes_desktop::mcp_host::read_framed_message(&mut reader);
}
