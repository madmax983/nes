use nes_core::mapper::{Nrom, Uxrom};

#[test]
fn nrom_ignores_bank_switch_writes() {
    let mut m = Nrom::new_32k();
    let before = m.read_prg(0x8000);
    m.write_prg(0x8000, 1);
    assert_eq!(before, m.read_prg(0x8000));
}

#[test]
fn uxrom_switches_lower_bank_only() {
    let mut m = Uxrom::new(8);
    m.write_prg(0x8000, 3);
    assert_eq!(m.selected_bank(), 3);
}
