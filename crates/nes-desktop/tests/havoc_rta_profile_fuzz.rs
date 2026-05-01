use nes_desktop::rta::RtaProfile;
use proptest::prelude::*;

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100000))]
    #[test]
    #[ignore = "havoc target"]
    fn havoc_fuzz_rta_profile_deserialize(data in proptest::collection::vec(any::<u8>(), 0..1024)) {
        let _ = toml::from_str::<RtaProfile>(&String::from_utf8_lossy(&data));
    }
}
