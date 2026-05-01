use nes_desktop::manual_state;
use proptest::prelude::*;
use std::path::Path;

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100000))]
    #[test]
    #[ignore = "havoc target"]
    fn havoc_fuzz_manual_state_hash_prefix(rom_hash in ".*") {
        let _ = manual_state::slot_path_for_rom(Path::new("dummy"), &rom_hash, 1);
    }
}
