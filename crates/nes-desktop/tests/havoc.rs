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
    #[should_panic]
    #[ignore = "Havoc Proptest Overflow Attack"]
    fn havoc_test_parse_args_overflow(
        latency in "\\d{20,100}" // An overly large number that causes panic on unwrap because it overflows the `u32`
    ) {
        let args = vec![
            "--netplay-delay".to_owned(),
            latency
        ];
        // The proptest finds strings that correctly parse regex but panic on u32 cast
        let _ = parse_runtime_args(&args).unwrap();
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
