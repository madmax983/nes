use nes_core::mapper::Fme7;

const PRG_8K: usize = 8 * 1024;
const CHR_1K: usize = 1024;
const CHR_8K: usize = 8 * 1024;

const WRAM_SELECT: u8 = 0x40;
const WRAM_ENABLE: u8 = 0x80;
const IRQ_COUNTER_ENABLE: u8 = 0x01;
const IRQ_ENABLE: u8 = 0x80;
const DOTS_PER_CPU_CYCLE: u32 = 3;

fn prg_with_bank_markers(banks: usize) -> Vec<u8> {
    let mut prg = vec![0_u8; banks * PRG_8K];
    for bank in 0..banks {
        prg[bank * PRG_8K] = bank as u8;
    }
    prg
}

fn chr_with_bank_markers(banks: usize) -> Vec<u8> {
    let mut chr = vec![0_u8; banks * CHR_1K];
    for bank in 0..banks {
        chr[bank * CHR_1K] = bank as u8;
    }
    chr
}

/// Two-step command/parameter write: select register `reg`, then write `value`.
fn select(m: &mut Fme7, reg: u8, value: u8) {
    m.write_prg(0x8000, reg);
    m.write_prg(0xA000, value);
}

fn pump_cpu_cycles(m: &mut Fme7, count: u32) {
    for _ in 0..count * DOTS_PER_CPU_CYCLE {
        m.on_ppu_dot(0, 0, false, 0);
    }
}

#[test]
fn fme7_prg_banks_switch_with_fixed_last_at_e000() {
    let mut m = Fme7::from_prg_chr(prg_with_bank_markers(8), chr_with_bank_markers(8));
    select(&mut m, 0x9, 3); // $8000 -> bank 3
    select(&mut m, 0xA, 5); // $A000 -> bank 5
    select(&mut m, 0xB, 6); // $C000 -> bank 6
    assert_eq!(m.read_prg(0x8000), 3);
    assert_eq!(m.read_prg(0xA000), 5);
    assert_eq!(m.read_prg(0xC000), 6);
    assert_eq!(m.read_prg(0xE000), 7); // fixed to last (8 - 1)
}

#[test]
fn fme7_6000_rom_mapped_returns_selected_bank_and_ignores_writes() {
    let mut m = Fme7::from_prg_chr(prg_with_bank_markers(8), chr_with_bank_markers(8));
    select(&mut m, 0x8, 4); // bit6 = 0 -> PRG-ROM bank 4 at $6000
    assert_eq!(m.read_prg_ram(0x6000), Some(4));
    assert_eq!(m.read_prg(0x6000), 4);
    m.write_prg(0x6000, 0xAA); // ignored while ROM-mapped
    assert_eq!(m.read_prg(0x6000), 4);
}

#[test]
fn fme7_6000_ram_round_trips_and_open_bus_when_disabled() {
    let mut m = Fme7::from_prg_chr(prg_with_bank_markers(8), chr_with_bank_markers(8));
    // RAM selected + enabled.
    select(&mut m, 0x8, WRAM_SELECT | WRAM_ENABLE);
    m.write_prg_ram(0x6000, 0x5A);
    assert_eq!(m.read_prg_ram(0x6000), Some(0x5A));

    // RAM selected but disabled: reads 0xFF, writes ignored.
    select(&mut m, 0x8, WRAM_SELECT);
    assert_eq!(m.read_prg_ram(0x6000), Some(0xFF));
    m.write_prg(0x6000, 0x12);

    // Re-enable: original byte survived the disabled window.
    select(&mut m, 0x8, WRAM_SELECT | WRAM_ENABLE);
    assert_eq!(m.read_prg(0x6000), 0x5A);
}

#[test]
fn fme7_chr_window_assembles_from_1k_banks() {
    let mut m = Fme7::from_prg_chr(prg_with_bank_markers(4), chr_with_bank_markers(16));
    select(&mut m, 0x0, 10);
    select(&mut m, 0x7, 15);
    let window = m.chr_window();
    assert_eq!(window[0x0000], 10);
    assert_eq!(window[0x1C00], 15);
}

#[test]
fn fme7_mirroring_register_produces_four_distinct_modes() {
    // `NametableMirroring` is not publicly re-exported, so assert the four
    // reg-0xC values map to four distinct modes without naming the enum. The
    // exact value->mode mapping is checked in the inline unit test.
    let mut m = Fme7::from_prg_chr(prg_with_bank_markers(4), chr_with_bank_markers(8));
    select(&mut m, 0xC, 0);
    let vertical = m.mirroring();
    select(&mut m, 0xC, 1);
    let horizontal = m.mirroring();
    select(&mut m, 0xC, 2);
    let one_lower = m.mirroring();
    select(&mut m, 0xC, 3);
    let one_upper = m.mirroring();

    assert_ne!(vertical, horizontal);
    assert_ne!(one_lower, one_upper);
    assert_ne!(vertical, one_lower);
    assert_ne!(horizontal, one_upper);

    // Selecting mode 0 again returns the same value.
    select(&mut m, 0xC, 0);
    assert_eq!(m.mirroring(), vertical);
}

#[test]
fn fme7_irq_fires_at_underflow_and_acks_on_reg_d_write() {
    let mut m = Fme7::from_prg_chr(prg_with_bank_markers(4), chr_with_bank_markers(8));
    const N: u16 = 8;
    select(&mut m, 0xE, (N & 0xFF) as u8);
    select(&mut m, 0xF, (N >> 8) as u8);
    select(&mut m, 0xD, IRQ_COUNTER_ENABLE | IRQ_ENABLE);

    // After N CPU cycles the counter is 0 but has not underflowed yet.
    pump_cpu_cycles(&mut m, u32::from(N));
    assert!(!m.irq_pending());

    // The (N+1)-th CPU cycle underflows $0000 -> $FFFF and asserts the IRQ.
    pump_cpu_cycles(&mut m, 1);
    assert!(m.irq_pending());

    // Writing reg 0xD acknowledges the pending IRQ.
    select(&mut m, 0xD, IRQ_COUNTER_ENABLE | IRQ_ENABLE);
    assert!(!m.irq_pending());
}

#[test]
fn fme7_counter_disable_freezes_and_irq_disable_suppresses() {
    // Counter disabled (bit0 = 0): no assertion regardless of dots.
    let mut frozen = Fme7::from_prg_chr(prg_with_bank_markers(4), chr_with_bank_markers(8));
    select(&mut frozen, 0xE, 3);
    select(&mut frozen, 0xF, 0);
    select(&mut frozen, 0xD, IRQ_ENABLE); // IRQ enabled but not counting
    pump_cpu_cycles(&mut frozen, 20);
    assert!(!frozen.irq_pending());

    // Counter enabled but IRQ disabled (bit7 = 0): underflow does not assert.
    let mut suppressed = Fme7::from_prg_chr(prg_with_bank_markers(4), chr_with_bank_markers(8));
    select(&mut suppressed, 0xE, 2);
    select(&mut suppressed, 0xF, 0);
    select(&mut suppressed, 0xD, IRQ_COUNTER_ENABLE);
    pump_cpu_cycles(&mut suppressed, 3); // 2 -> 1 -> 0 -> underflow
    assert!(!suppressed.irq_pending());
}

#[test]
fn fme7_chr_ram_when_absent_is_writable() {
    let mut m = Fme7::from_prg_chr(prg_with_bank_markers(4), vec![]);
    assert!(m.chr_writable());
    let mut window = [0_u8; CHR_8K];
    window[0] = 0x77;
    m.sync_chr_ram_from_ppu_window(&window);
    assert_eq!(m.chr_window()[0], 0x77);
}

#[test]
fn fme7_normalizes_inputs_and_guards_out_of_range_access() {
    // Undersized/odd PRG padded to whole 8KB banks; undersized/odd CHR-ROM
    // padded up to a whole window and left non-writable.
    let mut m = Fme7::from_prg_chr(vec![0x11; PRG_8K + 3], vec![0x22; CHR_8K + 5]);
    assert!(!m.chr_writable());
    assert_eq!(m.chr_window().len(), CHR_8K);

    // A sync request on CHR-ROM is a no-op early return.
    let mut window = [0_u8; CHR_8K];
    window[0] = 0x7E;
    m.sync_chr_ram_from_ppu_window(&window);
    assert_ne!(m.chr_window()[0], 0x7E);

    // Reads below $6000 return open bus; $C000-$FFFF writes (5B audio) are no-ops.
    assert_eq!(m.read_prg(0x5000), 0xFF);
    m.write_prg(0xC000, 0x55); // audio command register: ignored
    m.write_prg(0xE000, 0x55); // audio data register: ignored
}
