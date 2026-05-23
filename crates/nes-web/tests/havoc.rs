use nes_web::WebRuntime;
use proptest::prelude::*;

proptest! {
    #![proptest_config(ProptestConfig::with_cases(5000))]
    #[test]
    #[ignore = "havoc target"]
    fn havoc_fuzz_web_api(
        key_code in ".*",
        button in ".*"
    ) {
        let mut emulator = WebRuntime::new();
        let _ = emulator.press_button(&button);
        let _ = emulator.release_button(&button);
        let _ = emulator.dispatch_dom_key(&key_code, true);
    }
}
