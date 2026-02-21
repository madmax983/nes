use nes_core::mapper::Mmc1;

#[test]
fn mmc1_resets_shift_register_on_bit7_write() {
    let mut m = Mmc1::new(16, 8);
    m.write_prg(0xE000, 0x80);
    assert!(m.shift_is_reset());
}
