use nes_dsl::{RomBuildOptions, assemble, build_ines_nrom_rom};
use proptest::prelude::*;

proptest! {
    #![proptest_config(ProptestConfig::with_cases(50000))]
    #[test]
    fn does_not_crash(s in "[a-zA-Z0-9_ \\n\\r\\t#$:,\"\\.=-]*") {
        if let Ok(_program) = assemble(&s) {
            let options = RomBuildOptions::default();
            let _ = build_ines_nrom_rom(&s, &options);
        }
    }

    #[test]
    fn test_arbitrary_strings(s in ".*") {
        if let Ok(_program) = assemble(&s) {
            let options = RomBuildOptions::default();
            let _ = build_ines_nrom_rom(&s, &options);
        }
    }
}
