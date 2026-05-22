#![cfg(feature = "mcp-host")]

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

#[cfg(feature = "mcp-host")]
#[test]
#[should_panic(expected = "capacity overflow")]
#[ignore = "havoc target"]
fn havoc_mcp_host_oom_on_massive_content_length() {
    use std::io::{BufReader, Cursor};
    // The vulnerability is in `read_framed_message` inside `mcp_host.rs`.
    // It allocates `vec![0_u8; len]` based solely on the Content-Length header, without limits.
    let mut reader = BufReader::new(PanicOnInfiniteEofReader::new(Cursor::new(
        b"Content-Length: 18446744073709551615\r\n\r\n".to_vec()),
    ));
    let _ = nes_desktop::mcp_host::read_framed_message(&mut reader);
}
