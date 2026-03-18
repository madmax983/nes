use nes_core::mapper::Mmc1;

#[test]
fn mmc1_resets_shift_register_on_bit7_write() {
    let mut m = Mmc1::new(16, 8);
    m.write_prg(0xE000, 0x80);
    assert!(m.shift_is_reset());
}

#[test]
fn mmc1_state_can_be_saved_and_restored() {
    let mut mapper = Mmc1::new(16, 8);
    // Write 5 times to register 3 (PRG bank) to set it
    mapper.write_prg(0xE000, 0x80); // Reset
    mapper.write_prg(0xE000, 0x01);
    mapper.write_prg(0xE000, 0x00);
    mapper.write_prg(0xE000, 0x01);
    mapper.write_prg(0xE000, 0x00);
    mapper.write_prg(0xE000, 0x00); // PRG bank = 0b00101 = 5

    let state = mapper.state();
    assert_eq!(mapper.selected_prg_bank(), 5);

    mapper.write_prg(0xE000, 0x80); // Reset
    mapper.write_prg(0xE000, 0x00);
    mapper.write_prg(0xE000, 0x01);
    mapper.write_prg(0xE000, 0x00);
    mapper.write_prg(0xE000, 0x00);
    mapper.write_prg(0xE000, 0x00); // PRG bank = 0b00010 = 2
    assert_eq!(mapper.selected_prg_bank(), 2);

    mapper.restore_state(state);
    assert_eq!(mapper.selected_prg_bank(), 5);
}

#[test]
fn mmc1_read_prg_through_trait() {
    use nes_core::mapper::Mapper;
    let mut prg = vec![0_u8; 32 * 1024];
    prg[0x0000] = 0xAA;
    let mapper = nes_core::mapper::Mmc1::from_prg_rom(prg, 1);

    // Explicitly test <Self as Mapper>::read_prg
    assert_eq!(
        <nes_core::mapper::Mmc1 as Mapper>::read_prg(&mapper, 0x8000),
        mapper.read_prg(0x8000)
    );
}
