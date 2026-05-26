use nes_web::WebRuntime;
use proptest::prelude::*;

proptest! {
    #[test]
    #[ignore = "havoc target"]
    fn havoc_test_web_runtime_dispatch_dom_key(
        key_code in any::<String>(),
        pressed in any::<bool>()
    ) {
        let mut runtime = WebRuntime::new();
        let _ = runtime.dispatch_dom_key(&key_code, pressed);
    }

    #[test]
    #[ignore = "havoc target"]
    fn havoc_test_web_runtime_press_button(
        button in any::<String>()
    ) {
        let mut runtime = WebRuntime::new();
        let _ = runtime.press_button(&button);
    }
}
