use nes_core::{Command, CoreError, NesCore};

fn sample_nrom16_ines() -> Vec<u8> {
    let mut rom = vec![0_u8; 16 + 16 * 1024];
    rom[0] = 0x4E; // N
    rom[1] = 0x45; // E
    rom[2] = 0x53; // S
    rom[3] = 0x1A;
    rom[4] = 1; // 1 x 16KB PRG
    rom[5] = 0; // 0 x 8KB CHR

    let prg_start = 16;
    rom[prg_start] = 0xA9; // LDA #$42
    rom[prg_start + 1] = 0x42;

    // Reset vector at CPU $FFFC -> mirrored bank offset $3FFC.
    rom[prg_start + 0x3FFC] = 0x00;
    rom[prg_start + 0x3FFD] = 0x80;
    rom
}

#[test]
fn load_ines_rom_maps_prg_and_respects_reset_vector() {
    let mut core = NesCore::new();
    let info = core.load_ines_rom(&sample_nrom16_ines()).unwrap();

    assert_eq!(info.mapper_id, 0);
    assert_eq!(info.prg_rom_bytes, 16 * 1024);
    assert_eq!(core.cpu_pc(), 0x8000);

    core.execute(Command::StepCpu).unwrap();
    assert_eq!(core.cpu_a(), 0x42);
    assert_eq!(core.cpu_pc(), 0x8002);
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
