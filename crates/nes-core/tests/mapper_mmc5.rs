//! NesCore-level integration tests for mapper 5 (MMC5). These drive a synthetic
//! iNES mapper-5 image through the real loader / `build_mapper` path and exercise
//! the per-mapper fan-out arms in `api.rs`: PRG/PRG-RAM banking visible to the
//! CPU, the `$5000-$5FFF` expansion register write/read routing (multiplier and
//! ExRAM), the scanline IRQ driving the CPU IRQ vector, CHR banking through the
//! PPU render path, and save-state round-tripping.

use nes_core::{Command, FRAME_WIDTH, NesCore};

/// Builds a minimal valid iNES image for mapper 5 with `prg_16k` 16KB PRG banks
/// and `chr_8k` 8KB CHR banks (0 = CHR-RAM).
fn ines(prg_16k: u8, chr_8k: u8) -> Vec<u8> {
    let prg_len = prg_16k as usize * 16 * 1024;
    let chr_len = chr_8k as usize * 8 * 1024;
    let mut rom = vec![0_u8; 16 + prg_len + chr_len];
    rom[0] = 0x4E; // N
    rom[1] = 0x45; // E
    rom[2] = 0x53; // S
    rom[3] = 0x1A;
    rom[4] = prg_16k;
    rom[5] = chr_8k;
    rom[6] = (5 & 0x0F) << 4;
    rom[7] = 5 & 0xF0;
    rom
}

/// Installs a `NOP; JMP self` spin loop at the start of the fixed high 8KB PRG
/// bank ($E000) and points the reset vector at it.
fn install_spin_loop(rom: &mut [u8], prg_len: usize) -> u16 {
    let bank_size = 8 * 1024;
    let spin_addr: u16 = (0x10000 - bank_size) as u16; // $E000
    let bank_start = 16 + prg_len - bank_size;
    rom[bank_start] = 0xEA; // NOP
    rom[bank_start + 1] = 0x4C; // JMP abs
    rom[bank_start + 2] = spin_addr as u8;
    rom[bank_start + 3] = (spin_addr >> 8) as u8;
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

#[test]
fn mmc5_prg_bank_switch_through_nescore() {
    // 64KB PRG (eight 8KB banks). Mark $8000 of each 8KB bank.
    let mut rom = ines(4, 1);
    let prg_len = 4 * 16 * 1024;
    let bank_8k = 8 * 1024;
    for bank in 0..8 {
        rom[16 + bank * bank_8k] = 0x60 + bank as u8;
    }
    install_spin_loop(&mut rom, prg_len);

    let mut core = NesCore::new();
    let info = core.load_ines_rom(&rom).unwrap();
    assert_eq!(info.mapper_id, 5);

    // Power-on default (PRG mode 3): $8000 <- ROM bank 0.
    assert_eq!(core.read_memory(0x8000), 0x60);

    for _ in 0..8 {
        core.execute(Command::StepCpu).unwrap();
    }

    // PRG mode 3, select $8000 <- ROM bank 5 ($5114, bit7 = ROM).
    core.write_cpu_bus(0x5100, 3);
    core.write_cpu_bus(0x5114, 0x80 | 5);
    assert_eq!(core.read_memory(0x8000), 0x65);
    // $A000 <- ROM bank 2.
    core.write_cpu_bus(0x5115, 0x80 | 2);
    assert_eq!(core.read_memory(0xA000), 0x62);
}

#[test]
fn mmc5_prg_ram_write_protect_handshake_through_nescore() {
    let mut rom = ines(4, 1);
    let prg_len = 4 * 16 * 1024;
    install_spin_loop(&mut rom, prg_len);

    let mut core = NesCore::new();
    core.load_ines_rom(&rom).unwrap();

    // Without the $5102/$5103 handshake, PRG-RAM writes are dropped.
    core.write_cpu_bus(0x6000, 0xAB);
    assert_eq!(core.read_memory(0x6000), 0x00);

    // Enable writes (both magic values required), then round-trip.
    core.write_cpu_bus(0x5102, 0b10);
    core.write_cpu_bus(0x5103, 0b01);
    core.write_cpu_bus(0x6000, 0xAB);
    core.write_cpu_bus(0x7FFF, 0xCD);
    assert_eq!(core.read_memory(0x6000), 0xAB);
    assert_eq!(core.read_memory(0x7FFF), 0xCD);

    // A different PRG-RAM bank ($5113) sees separate storage.
    core.write_cpu_bus(0x5113, 1);
    assert_eq!(core.read_memory(0x6000), 0x00);
    core.write_cpu_bus(0x6000, 0x77);
    assert_eq!(core.read_memory(0x6000), 0x77);
    core.write_cpu_bus(0x5113, 0);
    assert_eq!(core.read_memory(0x6000), 0xAB);
}

#[test]
fn mmc5_multiplier_and_exram_through_nescore() {
    let mut rom = ines(4, 1);
    let prg_len = 4 * 16 * 1024;
    install_spin_loop(&mut rom, prg_len);

    let mut core = NesCore::new();
    core.load_ines_rom(&rom).unwrap();

    // 8x8 -> 16 unsigned multiplier: 17 * 15 = 255.
    core.write_cpu_bus(0x5205, 17);
    core.write_cpu_bus(0x5206, 15);
    assert_eq!(core.read_memory(0x5205), 255); // low byte
    assert_eq!(core.read_memory(0x5206), 0); // high byte

    // 200 * 4 = 800 = 0x0320.
    core.write_cpu_bus(0x5205, 200);
    core.write_cpu_bus(0x5206, 4);
    assert_eq!(core.read_memory(0x5205), 0x20);
    assert_eq!(core.read_memory(0x5206), 0x03);

    // ExRAM in CPU R/W mode round-trips through the CPU bus.
    core.write_cpu_bus(0x5104, 2); // ExRAM mode 2 (CPU R/W)
    core.write_cpu_bus(0x5C00, 0x3C);
    core.write_cpu_bus(0x5FFF, 0xD4);
    assert_eq!(core.read_memory(0x5C00), 0x3C);
    assert_eq!(core.read_memory(0x5FFF), 0xD4);

    // Switching ExRAM to read-only (mode 3) drops further writes.
    core.write_cpu_bus(0x5104, 3);
    core.write_cpu_bus(0x5C00, 0x99);
    assert_eq!(core.read_memory(0x5C00), 0x3C);
}

#[test]
fn mmc5_scanline_irq_vectors_cpu_through_nescore() {
    // Reset handler enables interrupts and spins; the IRQ handler loads a
    // sentinel. The MMC5 scanline IRQ (with rendering enabled) must vector the
    // CPU into the handler.
    let mut rom = ines(4, 1);
    let prg_len = 4 * 16 * 1024;
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

    // Enable background+sprite rendering so the MMC5 scanline counter advances.
    core.write_cpu_bus(0x2001, 0x18);
    // Fire the IRQ at scanline compare 8, and enable it.
    core.write_cpu_bus(0x5203, 8);
    core.write_cpu_bus(0x5204, 0x80);

    let mut vectored = false;
    for _ in 0..40_000 {
        core.execute(Command::StepCpu).unwrap();
        if core.cpu_a() == 0xAA {
            vectored = true;
            break;
        }
    }
    assert!(
        vectored,
        "MMC5 scanline IRQ should vector the CPU into the $E010 handler"
    );
}

/// Builds an MMC5 ROM whose two 8KB CHR banks render tile `$01` in visually
/// distinct palette indices, with a spin loop at $E000.
fn build_chr_bank_rom() -> Vec<u8> {
    let mut rom = ines(4, 2); // 64KB PRG, 16KB CHR (two 8KB banks)
    let prg_len = 4 * 16 * 1024;
    install_spin_loop(&mut rom, prg_len);

    let chr = 16 + prg_len;
    let bank_8k = 8 * 1024;
    // Bank 0, tile $01 -> plane0 set => color index 1.
    for i in 0..8 {
        rom[chr + 0x10 + i] = 0xFF;
    }
    // Bank 1, tile $01 -> plane1 set => color index 2.
    for i in 0..8 {
        rom[chr + bank_8k + 0x18 + i] = 0xFF;
    }
    rom
}

fn render_pixel_80_with_chr_bank(chr_bank: u8) -> [u8; 3] {
    let mut core = NesCore::new();
    core.load_ines_rom(&build_chr_bank_rom()).unwrap();

    core.write_cpu_bus(0x5101, 0); // CHR mode 0 (8KB)
    core.write_cpu_bus(0x5127, chr_bank); // $5127 selects the 8KB CHR bank
    core.write_cpu_bus(0x2000, 0x00); // PPUCTRL: BG pattern table $0000

    write_ppu_data(&mut core, 0x3F00, &[0x0F, 0x30, 0x16, 0x00]); // palette
    write_ppu_data(&mut core, 0x2000, &[0x01_u8; 32]); // nametable row 0 of tile $01

    core.write_cpu_bus(0x2005, 0x00);
    core.write_cpu_bus(0x2005, 0x00);
    core.write_cpu_bus(0x2001, 0x0A); // show background (+ leftmost 8px)

    core.execute(Command::StepFrame).unwrap();
    let frame = core.framebuffer_rgba();
    pixel_rgb(&frame, 80, 0)
}

#[test]
fn mmc5_chr_bank_switch_changes_rendered_pixels() {
    let bank0 = render_pixel_80_with_chr_bank(0);
    let bank1 = render_pixel_80_with_chr_bank(1);
    assert_ne!(
        bank0, bank1,
        "selecting a different MMC5 CHR bank must change the rendered output \
         (bank0={bank0:?}, bank1={bank1:?})"
    );
}

#[test]
fn mmc5_state_hash_and_save_state_round_trip() {
    let mut rom = ines(4, 1);
    let prg_len = 4 * 16 * 1024;
    let bank_8k = 8 * 1024;
    for bank in 0..8 {
        rom[16 + bank * bank_8k] = 0x60 + bank as u8;
    }
    install_spin_loop(&mut rom, prg_len);

    let mut core = NesCore::new();
    core.load_ines_rom(&rom).unwrap();
    core.write_cpu_bus(0x5100, 3); // PRG mode 3

    let h0 = core.state_hash();
    core.write_cpu_bus(0x5114, 0x80 | 4); // $8000 <- ROM bank 4
    let h1 = core.state_hash();
    assert_ne!(h0, h1, "a PRG bank write must change the state hash");
    assert_eq!(core.read_memory(0x8000), 0x64);

    // Snapshot, mutate, restore: the earlier hash and banking must return.
    let snap = core.save_state();
    core.write_cpu_bus(0x5114, 0x80 | 6); // bank 6 (bank 7 holds the spin loop)
    assert_ne!(core.state_hash(), h1);
    assert_eq!(core.read_memory(0x8000), 0x66);

    core.load_state(&snap);
    assert_eq!(
        core.state_hash(),
        h1,
        "restoring the snapshot must reproduce the earlier MMC5 state hash"
    );
    assert_eq!(core.read_memory(0x8000), 0x64);
}

#[test]
fn mmc5_prg_ram_save_state_round_trip() {
    let mut rom = ines(4, 1);
    let prg_len = 4 * 16 * 1024;
    install_spin_loop(&mut rom, prg_len);

    let mut core = NesCore::new();
    core.load_ines_rom(&rom).unwrap();

    core.write_cpu_bus(0x5102, 0b10);
    core.write_cpu_bus(0x5103, 0b01);
    core.write_cpu_bus(0x6000, 0x5A);

    let snap = core.save_state();
    core.write_cpu_bus(0x6000, 0x00);
    core.load_state(&snap);
    assert_eq!(core.read_memory(0x6000), 0x5A);
}
