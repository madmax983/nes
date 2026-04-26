#[cfg(target_arch = "wasm32")]
mod wasm_tests {
    use wasm_bindgen_test::*;
    use nes_web::NesWebEmulator;

    // don't configure test environment (so it defaults to node or is node compatible)

    #[wasm_bindgen_test]
    fn test_to_js_error_returns_string() {
        let mut emu = NesWebEmulator::new();
        // Trigger an error to test the `to_js_error` behavior
        let err_val = emu.load_rom(&[]).unwrap_err();

        assert!(err_val.is_string());
        let err_str = err_val.as_string().unwrap();
        assert_eq!(err_str, "failed to load rom: rom load failed: truncated ROM: expected at least 16 bytes, got 0");
    }
}
