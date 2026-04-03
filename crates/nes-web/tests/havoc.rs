use nes_web::WebRuntime;

#[test]
#[ignore = "Havoc Target"]
fn havoc_test_parse_button_dos() {
    let mut runtime = WebRuntime::new();
    let huge_str = "A".repeat(10_000_000);
    let _ = runtime.press_button(&huge_str);
}
