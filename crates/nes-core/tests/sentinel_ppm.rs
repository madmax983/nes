use nes_core::ppm::encode_ppm;

#[test]
fn encode_ppm_emits_expected_headers_and_pixel_layout_strict() {
    let ppm = encode_ppm(1, 1, &[255, 128, 64, 255]).unwrap();
    assert_eq!(ppm, b"P6\n1 1\n255\n\xFF\x80\x40");
}
