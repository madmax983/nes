use nes_core::{Command, CoreError, NesCore};

fn sample_ines(mapper_id: u8, prg_banks: u8) -> Vec<u8> {
    let mut rom = vec![0_u8; 16 + prg_banks as usize * 16 * 1024];
    rom[0] = 0x4E; // N
    rom[1] = 0x45; // E
    rom[2] = 0x53; // S
    rom[3] = 0x1A;
    rom[4] = prg_banks;
    rom[5] = 0; // 0 x 8KB CHR
    rom[6] = (mapper_id & 0x0F) << 4;
    rom[7] = mapper_id & 0xF0;

    rom
}

fn sample_nes2(mapper_id: u8, prg_banks: u8, chr_banks: u8) -> Vec<u8> {
    let mut rom = vec![0_u8; 16 + prg_banks as usize * 16 * 1024 + chr_banks as usize * 8 * 1024];
    rom[0] = 0x4E; // N
    rom[1] = 0x45; // E
    rom[2] = 0x53; // S
    rom[3] = 0x1A;
    rom[4] = prg_banks;
    rom[5] = chr_banks;
    rom[6] = (mapper_id & 0x0F) << 4;
    rom[7] = (mapper_id & 0xF0) | 0x08; // NES 2.0 marker
    rom[8] = 0; // no extended mapper bits
    rom[9] = 0; // no extended PRG/CHR size bits
    rom
}

#[test]
fn load_nrom16_maps_prg_and_respects_reset_vector() {
    let mut rom = sample_ines(0, 1);
    let prg_start = 16;

    rom[prg_start] = 0xA9; // LDA #$42
    rom[prg_start + 1] = 0x42;

    // Reset vector at CPU $FFFC -> mirrored bank offset $3FFC.
    rom[prg_start + 0x3FFC] = 0x00;
    rom[prg_start + 0x3FFD] = 0x80;

    let mut core = NesCore::new();
    let info = core.load_ines_rom(&rom).unwrap();

    assert_eq!(info.mapper_id, 0);
    assert_eq!(info.prg_rom_bytes, 16 * 1024);
    assert_eq!(core.cpu_pc(), 0x8000);

    core.execute(Command::StepCpu).unwrap();
    assert_eq!(core.cpu_a(), 0x42);
    assert_eq!(core.cpu_pc(), 0x8002);
}

#[test]
fn load_nes2_nrom32_maps_prg_and_respects_reset_vector() {
    let mut rom = sample_nes2(0, 2, 1);
    let prg_start = 16;
    let bank_size = 16 * 1024;

    // First bank ($8000): LDA #$42
    rom[prg_start] = 0xA9;
    rom[prg_start + 1] = 0x42;

    // Second bank ($C000): LDA #$99; reset vector -> $C000
    let second_bank = prg_start + bank_size;
    rom[second_bank] = 0xA9;
    rom[second_bank + 1] = 0x99;
    rom[second_bank + 0x3FFC] = 0x00;
    rom[second_bank + 0x3FFD] = 0xC0;

    let mut core = NesCore::new();
    let info = core.load_ines_rom(&rom).unwrap();

    assert_eq!(info.mapper_id, 0);
    assert_eq!(info.prg_rom_bytes, 32 * 1024);
    assert_eq!(core.cpu_pc(), 0xC000);

    core.execute(Command::StepCpu).unwrap();
    assert_eq!(core.cpu_a(), 0x99);
}

#[test]
fn load_uxrom_maps_first_and_last_bank_for_boot() {
    let mut rom = sample_ines(2, 3);
    let prg_start = 16;
    let bank_size = 16 * 1024;

    // First bank ($8000): LDA #$11
    rom[prg_start] = 0xA9;
    rom[prg_start + 1] = 0x11;

    // Last bank ($C000): LDA #$99 + reset vector -> $C000
    let last_bank = prg_start + 2 * bank_size;
    rom[last_bank] = 0xA9;
    rom[last_bank + 1] = 0x99;
    rom[last_bank + 0x3FFC] = 0x00;
    rom[last_bank + 0x3FFD] = 0xC0;

    let mut core = NesCore::new();
    let info = core.load_ines_rom(&rom).unwrap();

    assert_eq!(info.mapper_id, 2);
    assert_eq!(core.cpu_pc(), 0xC000);
    assert_eq!(core.read_memory(0x8001), 0x11);

    core.execute(Command::StepCpu).unwrap();
    assert_eq!(core.cpu_a(), 0x99);
    assert_eq!(core.cpu_pc(), 0xC002);
}

#[test]
fn uxrom_bank_switch_via_cpu_prg_write_changes_lower_window() {
    let mut rom = sample_ines(2, 3);
    let prg_start = 16;
    let bank_size = 16 * 1024;

    // Bank 0 callable function at $8000: LDA #$11 ; RTS
    rom[prg_start] = 0xA9;
    rom[prg_start + 1] = 0x11;
    rom[prg_start + 2] = 0x60;

    // Bank 1 callable function at $8000: LDA #$2A ; RTS
    let bank1 = prg_start + bank_size;
    rom[bank1] = 0xA9;
    rom[bank1 + 1] = 0x2A;
    rom[bank1 + 2] = 0x60;

    // Last bank reset program:
    // LDA #$01 ; STA $8000 ; JSR $8000
    let last_bank = prg_start + 2 * bank_size;
    rom[last_bank] = 0xA9;
    rom[last_bank + 1] = 0x01;
    rom[last_bank + 2] = 0x8D;
    rom[last_bank + 3] = 0x00;
    rom[last_bank + 4] = 0x80;
    rom[last_bank + 5] = 0x20;
    rom[last_bank + 6] = 0x00;
    rom[last_bank + 7] = 0x80;

    rom[last_bank + 0x3FFC] = 0x00;
    rom[last_bank + 0x3FFD] = 0xC0;

    let mut core = NesCore::new();
    core.load_ines_rom(&rom).unwrap();

    assert_eq!(core.read_memory(0x8001), 0x11);

    core.execute(Command::StepCpu).unwrap(); // LDA #$01
    core.execute(Command::StepCpu).unwrap(); // STA $8000 (switch bank)

    assert_eq!(core.read_memory(0x8001), 0x2A);

    core.execute(Command::StepCpu).unwrap(); // JSR $8000
    core.execute(Command::StepCpu).unwrap(); // LDA #$2A (bank 1)
    core.execute(Command::StepCpu).unwrap(); // RTS

    assert_eq!(core.cpu_a(), 0x2A);
    assert_eq!(core.cpu_pc(), 0xC008);
}

#[test]
fn load_mmc1_maps_first_and_last_bank_for_boot() {
    let mut rom = sample_ines(1, 4);
    let prg_start = 16;
    let bank_size = 16 * 1024;

    // First bank ($8000): LDA #$22
    rom[prg_start] = 0xA9;
    rom[prg_start + 1] = 0x22;

    // Last bank ($C000): LDA #$77 + reset vector -> $C000
    let last_bank = prg_start + 3 * bank_size;
    rom[last_bank] = 0xA9;
    rom[last_bank + 1] = 0x77;
    rom[last_bank + 0x3FFC] = 0x00;
    rom[last_bank + 0x3FFD] = 0xC0;

    let mut core = NesCore::new();
    let info = core.load_ines_rom(&rom).unwrap();

    assert_eq!(info.mapper_id, 1);
    assert_eq!(core.cpu_pc(), 0xC000);
    assert_eq!(core.read_memory(0x8001), 0x22);

    core.execute(Command::StepCpu).unwrap();
    assert_eq!(core.cpu_a(), 0x77);
    assert_eq!(core.cpu_pc(), 0xC002);
}

#[test]
fn mmc1_bank_switch_via_prg_writes_changes_lower_window() {
    let mut rom = sample_ines(1, 4);
    let prg_start = 16;
    let bank_size = 16 * 1024;

    // Bank 0 visible by default in lower window.
    rom[prg_start] = 0xA9;
    rom[prg_start + 1] = 0x10;

    // Bank 2 target value after switch.
    let bank2 = prg_start + 2 * bank_size;
    rom[bank2] = 0xA9;
    rom[bank2 + 1] = 0x42;

    // Reset vector -> $C000 in fixed last bank.
    let last_bank = prg_start + 3 * bank_size;
    rom[last_bank + 0x3FFC] = 0x00;
    rom[last_bank + 0x3FFD] = 0xC0;

    let mut core = NesCore::new();
    core.load_ines_rom(&rom).unwrap();

    assert_eq!(core.read_memory(0x8001), 0x10);

    // MMC1 serial write for value 0b00010 (bank 2), LSB-first.
    for bit in [0_u8, 1, 0, 0, 0] {
        core.write_cpu_bus(0xE000, bit);
    }

    assert_eq!(core.read_memory(0x8001), 0x42);
}

#[test]
fn invalid_ines_magic_is_rejected() {
    let mut core = NesCore::new();
    let err = core.load_ines_rom(&[0_u8; 16]).unwrap_err();

    match err {
        CoreError::RomLoadFailed(_) => {}
        other => panic!("unexpected error: {other:?}"),
    }
}

#[test]
fn unsupported_mapper_is_rejected() {
    let rom = sample_ines(4, 2);
    let mut core = NesCore::new();
    let err = core.load_ines_rom(&rom).unwrap_err();

    match err {
        CoreError::RomLoadFailed(_) => {}
        other => panic!("unexpected error: {other:?}"),
    }
}
