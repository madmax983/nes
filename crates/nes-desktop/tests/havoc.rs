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

#[test]
#[ignore = "Havoc OOM Attack (SIGKILL)"]
fn havoc_desktop_load_state_oom() {
    // The Trigger: passing /dev/zero to load_state_file will cause an OOM SIGKILL.
    // It blindly uses fs::read.
    let _ = nes_desktop::manual_state::load_state_file(std::path::Path::new("/dev/zero"), "hash");
}
