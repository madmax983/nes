use nes_dsl::{RomBuildOptions, assemble, build_ines_nrom_rom};
use proptest::prelude::*;

proptest! {
    #![proptest_config(ProptestConfig::with_cases(5000))]
    #[test]
    #[ignore = "havoc target"]
    fn havoc_fuzz_dsl(source in ".*") {
        let _ = assemble(&source);
        let _ = build_ines_nrom_rom(&source, &RomBuildOptions::default());
    }
}
