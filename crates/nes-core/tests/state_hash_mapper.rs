use nes_core::NesCore;

fn sample_ines(mapper_id: u8, prg_banks: u8) -> Vec<u8> {
    let mut rom = vec![0_u8; 16 + prg_banks as usize * 16 * 1024];
    rom[0] = 0x4E;
    rom[1] = 0x45;
    rom[2] = 0x53;
    rom[3] = 0x1A;
    rom[4] = prg_banks;
    rom[5] = 0;
    rom[6] = (mapper_id & 0x0F) << 4;
    rom[7] = mapper_id & 0xF0;
    rom
}

#[test]
fn state_hash_differentiates_mapper_state() {
    let empty_core = NesCore::new();
    let hash_empty = empty_core.state_hash();

    let mut uxrom_core = NesCore::new();
    let uxrom_rom = sample_ines(2, 4);
    uxrom_core.load_ines_rom(&uxrom_rom).unwrap();
    let hash_uxrom_initial = uxrom_core.state_hash();
    uxrom_core.write_cpu_bus(0x8000, 1);
    let hash_uxrom_bank1 = uxrom_core.state_hash();

    assert_ne!(hash_empty, hash_uxrom_initial);
    assert_ne!(hash_uxrom_initial, hash_uxrom_bank1);
}

#[test]
fn state_hash_differentiates_all_mappers() {
    let mut core = NesCore::new();
    let hash_empty = core.state_hash();

    // NROM
    core.load_ines_rom(&sample_ines(0, 1)).unwrap();
    let hash_nrom = core.state_hash();

    // UxROM
    core.load_ines_rom(&sample_ines(2, 2)).unwrap();
    let hash_uxrom = core.state_hash();

    // MMC1
    core.load_ines_rom(&sample_ines(1, 2)).unwrap();
    let hash_mmc1 = core.state_hash();

    // CNROM
    core.load_ines_rom(&sample_ines(3, 1)).unwrap();
    let hash_cnrom = core.state_hash();

    // AxROM
    core.load_ines_rom(&sample_ines(7, 2)).unwrap();
    let hash_axrom = core.state_hash();

    // GxROM
    core.load_ines_rom(&sample_ines(66, 2)).unwrap();
    let hash_gxrom = core.state_hash();

    // MMC3
    core.load_ines_rom(&sample_ines(4, 4)).unwrap();
    let hash_mmc3 = core.state_hash();

    let mut hashes = vec![
        hash_empty, hash_nrom, hash_uxrom, hash_mmc1, hash_cnrom, hash_axrom, hash_gxrom, hash_mmc3,
    ];
    hashes.sort();
    hashes.dedup();
    assert_eq!(
        hashes.len(),
        8,
        "All mapper types should produce unique initial state hashes"
    );
}
