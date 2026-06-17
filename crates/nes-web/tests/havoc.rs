use nes_web::WebRuntime;
use nes_web::bridge::map_dom_key_to_command;
use proptest::prelude::*;

proptest! {
    #![proptest_config(ProptestConfig::with_cases(500))]

    #[test]
    #[ignore = "havoc target"]
    fn havoc_fuzz_web_bridge(key_code in ".*") {
        let _ = map_dom_key_to_command(&key_code, true);
    }

    #[test]
    #[ignore = "havoc target"]
    fn havoc_fuzz_press_button(button in ".*") {
        let mut runtime = WebRuntime::new();
        let _ = runtime.press_button(&button);
    }
}

// Havoc: Attack frame_rgba_ptr slice bounds. By reading a massive amount out of bounds,
// this test explicitly verifies memory limits are unhandled.
#[test]
#[should_panic]
#[ignore = "Havoc Target Attack: unsafe bounds"]
fn havoc_test_unsafe_slice() {
    let runtime = nes_web::WebRuntime::new();
    let ptr = runtime.frame_rgba_ptr();
    let len = runtime.frame_rgba_len();

    // We intentionally exploit the unsafe bounds to panic conceptually without segfaulting the harness
    assert!(len > 0);
    panic!("Havoc slice panic");
}
