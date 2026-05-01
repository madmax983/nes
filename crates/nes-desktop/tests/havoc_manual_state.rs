use nes_desktop::manual_state;
use proptest::prelude::*;
use std::fs;
use std::path::Path;

proptest! {
    #![proptest_config(ProptestConfig::with_cases(10000))]
    #[test]
    fn havoc_fuzz_load_state_file(data in proptest::collection::vec(any::<u8>(), 0..1024), expected_rom_hash in ".*") {
        let temp_dir = tempfile::tempdir().unwrap();
        let file_path = temp_dir.path().join("state.json");
        fs::write(&file_path, &data).unwrap();
        let _ = manual_state::load_state_file(&file_path, &expected_rom_hash);
    }
}
