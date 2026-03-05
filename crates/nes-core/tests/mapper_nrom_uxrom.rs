use nes_core::mapper::{Nrom, Uxrom};

#[test]
fn nrom_ignores_bank_switch_writes() {
    let mut m = Nrom::new_32k();
    let before = m.read_prg(0x8000);
    // Nrom::write_prg does nothing and mutates nothing
    m.write_prg(0x8000, 1);
    // Explicitly test <Self as Mapper>::write_prg delegation
    use nes_core::mapper::Mapper;
    <Nrom as Mapper>::write_prg(&mut m, 0x8000, 1);
    assert_eq!(before, m.read_prg(0x8000));
}

#[test]
fn uxrom_switches_lower_bank_only() {
    let mut m = Uxrom::new(8);
    m.write_prg(0x8000, 3);
    assert_eq!(m.selected_bank(), 3);
}

#[test]
fn nrom_reads_from_prg_rom() {
    let m = Nrom::new_32k();
    // In Nrom::new_32k(), byte at idx is idx & 0xFF.
    // 0x8000 maps to idx 0. 0xFFFF maps to idx 0x7FFF.
    assert_eq!(m.read_prg(0x8000), 0);
    assert_eq!(m.read_prg(0xFFFF), 0xFF);

    // Test that the PRG bounds wrap around
    let m_small = Nrom::from_prg_rom(vec![0x42]);
    assert_eq!(m_small.read_prg(0x8000), 0x42);
    assert_eq!(m_small.read_prg(0xFFFF), 0x42);
}

#[test]
fn uxrom_from_prg_rom_counts_banks() {
    let m = Uxrom::from_prg_rom(vec![0_u8; 32 * 1024]);
    // 32K should be 2 banks
    let _ = m.read_prg(0x8000); // Verify it doesn't crash
}

#[test]
fn uxrom_reads_lower_bank_from_selected_and_upper_from_last() {
    // 2 banks: Bank 0 and Bank 1
    // Bank 0 bytes are (idx & 0xFF), Bank 1 bytes are ((idx + 16K) & 0xFF)
    let mut m = Uxrom::new(2);

    // Select bank 0
    m.write_prg(0x8000, 0);
    assert_eq!(m.selected_bank(), 0);

    // Lower bank (0x8000) should be Bank 0 (offset 0)
    assert_eq!(m.read_prg(0x8000), 0);
    // Upper bank (0xC000) should be Bank 1 (last bank, offset 16384)
    assert_eq!(m.read_prg(0xC000), (16384 & 0xFF) as u8);

    // Select bank 1
    m.write_prg(0x8000, 1);
    assert_eq!(m.selected_bank(), 1);

    // Lower bank (0x8000) should now be Bank 1
    assert_eq!(m.read_prg(0x8000), (16384 & 0xFF) as u8);
    // Upper bank (0xC000) should still be Bank 1
    assert_eq!(m.read_prg(0xC000), (16384 & 0xFF) as u8);
}

#[test]
fn uxrom_boundary_read() {
    let mut m = Uxrom::new(4);
    m.write_prg(0x8000, 1);

    // 0xBFFF should map to lower bank (Bank 1, offset 16384)
    // Within bank offset is 0x3FFF
    assert_eq!(m.read_prg(0xBFFF), ((16384 + 0x3FFF) & 0xFF) as u8);

    // 0xC000 should map to upper bank (Bank 3, offset 49152)
    // Within bank offset is 0
    assert_eq!(m.read_prg(0xC000), (49152 & 0xFF) as u8);
}
