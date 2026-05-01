use nes_desktop::rta;
use proptest::prelude::*;

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100000))]
    #[test]
    #[ignore = "havoc target"]
    fn havoc_fuzz_rta_compare_rom_hashes(a in ".*", b in ".*") {
        let _ = rta::compare_rom_hashes(&a, &b);
    }
}
