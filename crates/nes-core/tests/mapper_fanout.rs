//! NesCore-level integration tests for the newly added mappers (11, 71, 206,
//! 69, 9, 10). These drive a synthetic iNES ROM through the real
//! loader/`build_mapper` path and exercise the per-mapper fan-out arms in
//! `api.rs` (`read_prg`/`write_prg`/`read_prg_ram`/`write_prg_ram`,
//! `chr_window`/`mirroring_override`, `on_ppu_dot`/`irq_pending`,
//! `sync_chr_ram_from_ppu_window`, the CHR-fetch latch pump, `mapper_hash_component`,
//! and save-state clone/restore) that the direct mapper unit tests never reach.

use nes_core::{Command, FRAME_WIDTH, NesCore};

/// Builds a minimal valid iNES image for `mapper` with `prg_16k` 16KB PRG banks
/// and `chr_8k` 8KB CHR banks (0 = CHR-RAM). PRG/CHR payloads are zero-filled;
/// callers patch in code/markers afterwards.
fn ines(mapper: u8, prg_16k: u8, chr_8k: u8) -> Vec<u8> {
    let prg_len = prg_16k as usize * 16 * 1024;
    let chr_len = chr_8k as usize * 8 * 1024;
    let mut rom = vec![0_u8; 16 + prg_len + chr_len];
    rom[0] = 0x4E; // N
    rom[1] = 0x45; // E
    rom[2] = 0x53; // S
    rom[3] = 0x1A;
    rom[4] = prg_16k;
    rom[5] = chr_8k;
    rom[6] = (mapper & 0x0F) << 4;
    rom[7] = mapper & 0xF0;
    rom
}

/// Installs a `NOP; JMP self` spin loop at the start of the fixed high PRG bank
/// and points the reset vector at it. `bank_size` is the size of the fixed bank
/// mapped at the top of the address space (8KB -> $E000, 16KB -> $C000).
fn install_spin_loop(rom: &mut [u8], prg_len: usize, bank_size: usize) -> u16 {
    let spin_addr: u16 = (0x10000 - bank_size) as u16; // $E000 or $C000
    let bank_start = 16 + prg_len - bank_size;
    rom[bank_start] = 0xEA; // NOP
    rom[bank_start + 1] = 0x4C; // JMP abs
    rom[bank_start + 2] = spin_addr as u8;
    rom[bank_start + 3] = (spin_addr >> 8) as u8;
    // Reset vector at CPU $FFFC/$FFFD (top of the fixed bank).
    rom[16 + prg_len - 4] = spin_addr as u8;
    rom[16 + prg_len - 3] = (spin_addr >> 8) as u8;
    spin_addr
}

fn write_ppu_data(core: &mut NesCore, addr: u16, data: &[u8]) {
    core.write_cpu_bus(0x2006, (addr >> 8) as u8);
    core.write_cpu_bus(0x2006, addr as u8);
    for &byte in data {
        core.write_cpu_bus(0x2007, byte);
    }
}

fn pixel_rgb(frame: &[u8], x: usize, y: usize) -> [u8; 3] {
    let idx = (y * FRAME_WIDTH + x) * 4;
    [frame[idx], frame[idx + 1], frame[idx + 2]]
}

/// Exercises the common per-mapper runtime paths: state-hash changes on a bank
/// write, and a save-state snapshot/restore round-trip that re-materializes the
/// mapper PRG/CHR/mirroring windows.
fn assert_hash_and_save_state_round_trip(core: &mut NesCore, select_addr: u16, a: u8, b: u8) {
    let h0 = core.state_hash();
    core.write_cpu_bus(select_addr, a);
    let h1 = core.state_hash();
    assert_ne!(h0, h1, "a mapper bank write must change the state hash");

    let snap = core.save_state();
    core.write_cpu_bus(select_addr, b);
    assert_ne!(
        core.state_hash(),
        h1,
        "a second distinct bank write must change the state hash again"
    );

    core.load_state(&snap);
    assert_eq!(
        core.state_hash(),
        h1,
        "restoring the snapshot must reproduce the earlier mapper state hash"
    );
}

// --- Mapper 11 (Color Dreams) ----------------------------------------------

#[test]
fn colordreams_prg_bank_switch_through_nescore() {
    // 64KB PRG (two 32KB banks) + 8KB CHR-ROM.
    let mut rom = ines(11, 4, 1);

    // Boot bank 0 spans $8000-$FFFF; spin at $C000, reset vector -> $C000.
    let c000 = 16 + 0x4000;
    rom[c000] = 0xEA;
    rom[c000 + 1] = 0x4C;
    rom[c000 + 2] = 0x00;
    rom[c000 + 3] = 0xC0;
    rom[16 + 0x7FFC] = 0x00;
    rom[16 + 0x7FFD] = 0xC0;

    // Marker byte at $8000 in each 32KB bank.
    rom[16] = 0xA0; // bank 0
    rom[16 + 0x8000] = 0xA1; // bank 1

    let mut core = NesCore::new();
    let info = core.load_ines_rom(&rom).unwrap();
    assert_eq!(info.mapper_id, 11);
    assert_eq!(core.read_memory(0x8000), 0xA0);

    // A few spin-loop steps advance the hardware clock (mapper on_ppu_dot no-op).
    for _ in 0..8 {
        core.execute(Command::StepCpu).unwrap();
    }

    // bits 0-1 select the 32KB PRG bank; bits 4-7 the CHR bank.
    core.write_cpu_bus(0x8000, 0x01);
    assert_eq!(core.read_memory(0x8000), 0xA1);

    assert_hash_and_save_state_round_trip(&mut core, 0x8000, 0x00, 0x01);
}

// --- Mapper 71 (Camerica) ---------------------------------------------------

#[test]
fn camerica_prg_bank_switch_through_nescore() {
    // 64KB PRG (four 16KB banks), CHR-RAM.
    let mut rom = ines(71, 4, 0);
    let prg_len = 4 * 16 * 1024;
    install_spin_loop(&mut rom, prg_len, 16 * 1024); // fixed last 16KB at $C000

    // Switchable low window ($8000) markers per 16KB bank.
    rom[16] = 0x10; // bank 0
    rom[16 + 16 * 1024] = 0x11; // bank 1
    rom[16 + 2 * 16 * 1024] = 0x12; // bank 2

    let mut core = NesCore::new();
    let info = core.load_ines_rom(&rom).unwrap();
    assert_eq!(info.mapper_id, 71);
    assert_eq!(core.read_memory(0x8000), 0x10);

    for _ in 0..8 {
        core.execute(Command::StepCpu).unwrap();
    }

    // $C000-$FFFF writes select the switchable $8000 bank (low nibble).
    core.write_cpu_bus(0xC000, 0x02);
    assert_eq!(core.read_memory(0x8000), 0x12);
    core.write_cpu_bus(0xC000, 0x01);
    assert_eq!(core.read_memory(0x8000), 0x11);

    assert_hash_and_save_state_round_trip(&mut core, 0xC000, 0x02, 0x03);
}

// --- Mapper 206 (Namco-108) -------------------------------------------------

#[test]
fn namco108_prg_bank_switch_through_nescore() {
    // 64KB PRG (eight 8KB banks) + 8KB CHR-ROM.
    let mut rom = ines(206, 4, 1);
    let prg_len = 4 * 16 * 1024;
    let bank_8k = 8 * 1024;
    // Marker at $8000 of each 8KB bank.
    for bank in 0..8 {
        rom[16 + bank * bank_8k] = bank as u8;
    }
    install_spin_loop(&mut rom, prg_len, bank_8k); // $E000 fixed to last 8KB bank

    let mut core = NesCore::new();
    let info = core.load_ines_rom(&rom).unwrap();
    assert_eq!(info.mapper_id, 206);

    // Default: $C000/$E000 fixed to the last two banks (6, 7).
    assert_eq!(core.read_memory(0xC000), 6);

    for _ in 0..8 {
        core.execute(Command::StepCpu).unwrap();
    }

    // Select register 6 (-> $8000 slot) = bank 3.
    core.write_cpu_bus(0x8000, 6);
    core.write_cpu_bus(0x8001, 3);
    assert_eq!(core.read_memory(0x8000), 3);
    // Select register 7 (-> $A000 slot) = bank 4.
    core.write_cpu_bus(0x8000, 7);
    core.write_cpu_bus(0x8001, 4);
    assert_eq!(core.read_memory(0xA000), 4);

    // state_hash / save-state round trip driven by an R6 write.
    let h0 = core.state_hash();
    core.write_cpu_bus(0x8000, 6);
    core.write_cpu_bus(0x8001, 1);
    let h1 = core.state_hash();
    assert_ne!(h0, h1);
    let snap = core.save_state();
    core.write_cpu_bus(0x8001, 2);
    assert_ne!(core.state_hash(), h1);
    core.load_state(&snap);
    assert_eq!(core.state_hash(), h1);
}

// --- Mapper 69 (Sunsoft FME-7) ----------------------------------------------

#[test]
fn fme7_prg_and_wram_and_bank_switch_through_nescore() {
    // 32KB PRG (four 8KB banks) + 8KB CHR-ROM.
    let mut rom = ines(69, 2, 1);
    let prg_len = 2 * 16 * 1024;
    let bank_8k = 8 * 1024;
    for bank in 0..4 {
        rom[16 + bank * bank_8k] = 0x20 + bank as u8;
    }
    install_spin_loop(&mut rom, prg_len, bank_8k); // $E000 fixed to last bank

    let mut core = NesCore::new();
    let info = core.load_ines_rom(&rom).unwrap();
    assert_eq!(info.mapper_id, 69);
    assert_eq!(core.read_memory(0xE000), 0xEA); // spin NOP from fixed last bank

    for _ in 0..8 {
        core.execute(Command::StepCpu).unwrap();
    }

    // Select PRG reg 0x9 ($8000 slot) = bank 1 (bank 3 is the fixed last bank,
    // whose marker the spin loop overwrites).
    core.write_cpu_bus(0x8000, 0x09);
    core.write_cpu_bus(0xA000, 1);
    assert_eq!(core.read_memory(0x8000), 0x21);

    // WRAM: reg 0x8 = select+enable, then round-trip $6000-$7FFF through the CPU
    // bus (write_prg_ram / read_prg_ram fan-out arms).
    core.write_cpu_bus(0x8000, 0x08);
    core.write_cpu_bus(0xA000, 0xC0); // bit6 select + bit7 enable
    core.write_cpu_bus(0x6000, 0x5A);
    core.write_cpu_bus(0x7FFF, 0xC3);
    assert_eq!(core.read_memory(0x6000), 0x5A);
    assert_eq!(core.read_memory(0x7FFF), 0xC3);

    // A bank write must change the state hash (FME-7 mapper-hash fan-out arm).
    let h0 = core.state_hash();
    core.write_cpu_bus(0x8000, 0x09);
    core.write_cpu_bus(0xA000, 2);
    assert_ne!(h0, core.state_hash());

    // A save-state round trip must preserve the WRAM contents and bank state.
    let snap = core.save_state();
    core.write_cpu_bus(0x6000, 0x00);
    core.load_state(&snap);
    assert_eq!(core.read_memory(0x6000), 0x5A);
}

#[test]
fn fme7_irq_counter_underflow_vectors_cpu_through_nescore() {
    // 32KB PRG + 8KB CHR. Reset handler does CLI then spins; the IRQ vector
    // points at a handler that loads a sentinel. Stepping lets the FME-7 IRQ
    // counter underflow, which must vector the CPU into the handler.
    let mut rom = ines(69, 2, 1);
    let prg_len = 2 * 16 * 1024;
    let bank_8k = 8 * 1024;
    let bank_start = 16 + prg_len - bank_8k; // $E000

    // $E000: CLI; NOP; JMP $E001 (spin with interrupts enabled).
    rom[bank_start] = 0x58; // CLI
    rom[bank_start + 1] = 0xEA; // NOP
    rom[bank_start + 2] = 0x4C; // JMP $E001
    rom[bank_start + 3] = 0x01;
    rom[bank_start + 4] = 0xE0;
    // $E010: LDA #$AA; JMP $E010 (IRQ handler).
    rom[bank_start + 0x10] = 0xA9;
    rom[bank_start + 0x11] = 0xAA;
    rom[bank_start + 0x12] = 0x4C;
    rom[bank_start + 0x13] = 0x10;
    rom[bank_start + 0x14] = 0xE0;
    // Reset vector -> $E000, IRQ/BRK vector -> $E010.
    rom[16 + prg_len - 4] = 0x00;
    rom[16 + prg_len - 3] = 0xE0;
    rom[16 + prg_len - 2] = 0x10;
    rom[16 + prg_len - 1] = 0xE0;

    let mut core = NesCore::new();
    core.load_ines_rom(&rom).unwrap();

    // Program the IRQ counter: low = 10, high = 0, then enable count + IRQ.
    core.write_cpu_bus(0x8000, 0x0E);
    core.write_cpu_bus(0xA000, 10);
    core.write_cpu_bus(0x8000, 0x0F);
    core.write_cpu_bus(0xA000, 0);
    core.write_cpu_bus(0x8000, 0x0D);
    core.write_cpu_bus(0xA000, 0x81); // counter enable (bit0) + IRQ enable (bit7)

    let mut vectored = false;
    for _ in 0..200 {
        core.execute(Command::StepCpu).unwrap();
        if core.cpu_a() == 0xAA {
            vectored = true;
            break;
        }
    }
    assert!(
        vectored,
        "FME-7 IRQ counter underflow should vector the CPU into the $E010 handler"
    );
}

// --- Mapper 9 (MMC2) --------------------------------------------------------

#[test]
fn mmc2_prg_bank_switch_and_save_state_through_nescore() {
    // 32KB PRG (four 8KB banks) + 8KB CHR-ROM (two 4KB banks).
    let mut rom = ines(9, 2, 1);
    let prg_len = 2 * 16 * 1024;
    let bank_8k = 8 * 1024;
    for bank in 0..4 {
        rom[16 + bank * bank_8k] = 0x30 + bank as u8;
    }
    install_spin_loop(&mut rom, prg_len, bank_8k); // $E000 fixed to last bank

    let mut core = NesCore::new();
    let info = core.load_ines_rom(&rom).unwrap();
    assert_eq!(info.mapper_id, 9);

    for _ in 0..8 {
        core.execute(Command::StepCpu).unwrap();
    }

    // $A000 selects the switchable $8000-$9FFF 8KB bank.
    core.write_cpu_bus(0xA000, 1);
    assert_eq!(core.read_memory(0x8000), 0x31);

    assert_hash_and_save_state_round_trip(&mut core, 0xA000, 2, 3);
}

// --- Mapper 10 (MMC4) -------------------------------------------------------

#[test]
fn mmc4_prg_and_wram_and_save_state_through_nescore() {
    // 32KB PRG (two 16KB banks) + 8KB CHR-ROM (two 4KB banks).
    let mut rom = ines(10, 2, 1);
    let prg_len = 2 * 16 * 1024;
    rom[16] = 0x40; // switchable bank 0 marker at $8000
    install_spin_loop(&mut rom, prg_len, 16 * 1024); // $C000 fixed last 16KB

    let mut core = NesCore::new();
    let info = core.load_ines_rom(&rom).unwrap();
    assert_eq!(info.mapper_id, 10);
    assert_eq!(core.read_memory(0x8000), 0x40);

    for _ in 0..8 {
        core.execute(Command::StepCpu).unwrap();
    }

    // MMC4 PRG-RAM ($6000-$7FFF) is always enabled: round-trip through CPU bus.
    core.write_cpu_bus(0x6000, 0xAB);
    core.write_cpu_bus(0x7FFF, 0xCD);
    assert_eq!(core.read_memory(0x6000), 0xAB);
    assert_eq!(core.read_memory(0x7FFF), 0xCD);

    // A bank write must change the state hash (MMC4 mapper-hash fan-out arm).
    let h0 = core.state_hash();
    core.write_cpu_bus(0xA000, 1);
    assert_ne!(h0, core.state_hash());

    let snap = core.save_state();
    core.write_cpu_bus(0x6000, 0x00);
    core.write_cpu_bus(0xA000, 0);
    core.load_state(&snap);
    assert_eq!(core.read_memory(0x6000), 0xAB);
}

/// Builds a 32KB-PRG / 8KB-CHR MMC4 iNES image whose two 4KB CHR banks render
/// tile `$01` in visually distinct palette indices, with a `NOP; JMP self` spin
/// loop at `$C000` (the fixed last 16KB bank).
fn build_mmc4_latch_rom() -> Vec<u8> {
    const PRG_BYTES: usize = 32 * 1024;
    const CHR_4K: usize = 4 * 1024;
    let mut rom = ines(10, 2, 1);

    // Spin loop at $C000 (fixed last 16KB bank), reset vector -> $C000.
    let c000 = 16 + PRG_BYTES - 16 * 1024;
    rom[c000] = 0xEA;
    rom[c000 + 1] = 0x4C;
    rom[c000 + 2] = 0x00;
    rom[c000 + 3] = 0xC0;
    rom[16 + PRG_BYTES - 4] = 0x00;
    rom[16 + PRG_BYTES - 3] = 0xC0;

    // CHR tile $01 pattern at bank-relative offset $10.
    let chr = 16 + PRG_BYTES;
    // 4KB bank 0, tile $01 -> plane0 set, plane1 clear => color index 1.
    for i in 0..8 {
        rom[chr + 0x10 + i] = 0xFF;
    }
    // 4KB bank 1, tile $01 -> plane1 set => color index 2.
    for i in 0..8 {
        rom[chr + CHR_4K + 0x18 + i] = 0xFF;
    }
    rom
}

fn mmc4_render_row0_pixel_80(row0: &[u8; 32]) -> [u8; 3] {
    let mut core = NesCore::new();
    core.load_ines_rom(&build_mmc4_latch_rom()).unwrap();

    // Low-half CHR banks: latch0 == FD -> bank 1 ($B000), FE -> bank 0 ($C000).
    core.write_cpu_bus(0xB000, 1);
    core.write_cpu_bus(0xC000, 0);
    core.write_cpu_bus(0x2000, 0x00); // PPUCTRL: BG pattern table $0000, no NMI

    write_ppu_data(&mut core, 0x3F00, &[0x0F, 0x30, 0x16, 0x00]); // palette
    write_ppu_data(&mut core, 0x2000, row0); // nametable row 0

    core.write_cpu_bus(0x2005, 0x00);
    core.write_cpu_bus(0x2005, 0x00);
    core.write_cpu_bus(0x2001, 0x0A); // show background (+ leftmost 8px)

    core.execute(Command::StepFrame).unwrap();
    let frame = core.framebuffer_rgba();
    pixel_rgb(&frame, 80, 0)
}

#[test]
fn mmc4_chr_latch_rebanks_background_through_ppu_render_path() {
    // No trigger tile: latch0 stays at its power-on FE state (bank 0, index 1).
    let mut no_trigger = [0x01_u8; 32];
    no_trigger[0] = 0x01;
    let baseline = mmc4_render_row0_pixel_80(&no_trigger);

    // Top-left tile $FD: fetching it during rendering flips latch0 -> FD through
    // the PPU CHR-fetch hook, so later tile $01 renders from bank 1 (index 2).
    let mut with_trigger = [0x01_u8; 32];
    with_trigger[0] = 0xFD;
    let latched = mmc4_render_row0_pixel_80(&with_trigger);

    assert_ne!(
        baseline, latched,
        "the FD tile fetch should re-bank the MMC4 low CHR half through the PPU hook \
         (baseline={baseline:?}, latched={latched:?})"
    );
}
