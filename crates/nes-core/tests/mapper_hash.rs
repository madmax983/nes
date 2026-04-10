use nes_core::NesCore;

fn load_rom(core: &mut NesCore, mapper_id: u8, prg_banks: u8, chr_banks: u8) -> Vec<u8> {
    let prg_size = prg_banks as usize * 16 * 1024;
    let chr_size = chr_banks as usize * 8 * 1024;
    let mut rom = vec![0_u8; 16 + prg_size + chr_size];
    rom[0] = 0x4E;
    rom[1] = 0x45;
    rom[2] = 0x53;
    rom[3] = 0x1A;
    rom[4] = prg_banks;
    rom[5] = chr_banks;
    rom[6] = (mapper_id & 0x0F) << 4;
    rom[7] = mapper_id & 0xF0;

    // Fill with distinct data so bank switches map to distinct values
    for (i, byte) in rom.iter_mut().enumerate().skip(16) {
        *byte = (i % 256) as u8;
    }

    core.load_ines_rom(&rom).unwrap();
    rom
}

#[test]
fn nrom_hash_component() {
    let mut core = NesCore::new();
    load_rom(&mut core, 0, 1, 1);
    let hash = core.state_hash();
    assert_ne!(hash, NesCore::new().state_hash());
}

#[test]
fn uxrom_hash_component_depends_on_bank() {
    let mut core1 = NesCore::new();
    let rom = load_rom(&mut core1, 2, 4, 0);

    let mut core2 = NesCore::new();
    core2.load_ines_rom(&rom).unwrap();

    core1.write_cpu_bus(0x8000, 1); // switch bank
    assert_ne!(core1.state_hash(), core2.state_hash());
}

#[test]
fn mmc1_hash_component_depends_on_bank() {
    let mut core1 = NesCore::new();
    let rom = load_rom(&mut core1, 1, 4, 0);

    let mut core2 = NesCore::new();
    core2.load_ines_rom(&rom).unwrap();

    // Switch bank (requires 5 sequential writes)
    for _ in 0..5 {
        core1.write_cpu_bus(0xE000, 1);
    }

    assert_ne!(core1.state_hash(), core2.state_hash());
}

#[test]
fn cnrom_hash_component_depends_on_bank() {
    let mut core1 = NesCore::new();
    let rom = load_rom(&mut core1, 3, 1, 4);

    let mut core2 = NesCore::new();
    core2.load_ines_rom(&rom).unwrap();

    core1.write_cpu_bus(0x8000, 1);
    assert_ne!(core1.state_hash(), core2.state_hash());
}

#[test]
fn axrom_hash_component_depends_on_bank() {
    let mut core1 = NesCore::new();
    let rom = load_rom(&mut core1, 7, 4, 0);

    let mut core2 = NesCore::new();
    core2.load_ines_rom(&rom).unwrap();

    core1.write_cpu_bus(0x8000, 1);
    assert_ne!(core1.state_hash(), core2.state_hash());
}

#[test]
fn axrom_hash_component_depends_on_nametable() {
    let mut core1 = NesCore::new();
    let rom = load_rom(&mut core1, 7, 4, 0);

    let mut core2 = NesCore::new();
    core2.load_ines_rom(&rom).unwrap();

    core1.write_cpu_bus(0x8000, 0x10);
    assert_ne!(core1.state_hash(), core2.state_hash());
}

#[test]
fn gxrom_hash_component_depends_on_prg_bank() {
    let mut core1 = NesCore::new();
    let rom = load_rom(&mut core1, 66, 4, 4);

    let mut core2 = NesCore::new();
    core2.load_ines_rom(&rom).unwrap();

    core1.write_cpu_bus(0x8000, 0x10); // shift left 4 bits for PRG
    assert_ne!(core1.state_hash(), core2.state_hash());
}

#[test]
fn gxrom_hash_component_depends_on_chr_bank() {
    let mut core1 = NesCore::new();
    let rom = load_rom(&mut core1, 66, 4, 4);

    let mut core2 = NesCore::new();
    core2.load_ines_rom(&rom).unwrap();

    core1.write_cpu_bus(0x8000, 0x01); // lower 2 bits for CHR
    assert_ne!(core1.state_hash(), core2.state_hash());
}

#[test]
fn mmc3_hash_component_depends_on_banks() {
    let mut core1 = NesCore::new();
    // 4 PRG banks (64K total) - enough for switching, PRG starts at 16
    let mut rom1 = vec![0_u8; 16 + 4 * 16 * 1024];
    rom1[0] = 0x4E;
    rom1[1] = 0x45;
    rom1[2] = 0x53;
    rom1[3] = 0x1A;
    rom1[4] = 4;
    rom1[5] = 0;
    rom1[6] = (4 & 0x0F) << 4;
    rom1[7] = 4 & 0xF0;

    // Give different PRG pages different values so read_prg() will be different
    for (i, byte) in rom1.iter_mut().enumerate().skip(16) {
        *byte = (i / 1024) as u8;
    }

    core1.load_ines_rom(&rom1).unwrap();

    let mut core2 = NesCore::new();
    core2.load_ines_rom(&rom1).unwrap();

    // Select PRG ROM bank at 0x8000 (register 6)
    core1.write_cpu_bus(0x8000, 6);
    // Write bank number 1 (it is 8k bank so it will point to PRG ROM address 0x2000 which is part of 16K bank 0, but wait...)
    // Instead let's pick PRG bank 2 or 3 to get different values (value 1 or above)
    // 8K bank index 2 = address 16K inside ROM
    core1.write_cpu_bus(0x8001, 2);
    let _ = core1.read_memory(0x8000);

    assert_ne!(core1.state_hash(), core2.state_hash());
}

#[test]
fn gxrom_hash_component_combines_fields_with_xor() {
    let mut core1 = NesCore::new();
    let rom = load_rom(&mut core1, 66, 4, 4);

    let mut core_base = NesCore::new();
    core_base.load_ines_rom(&rom).unwrap();
    let hash_base = core_base.state_hash();

    let mut core_prg = NesCore::new();
    core_prg.load_ines_rom(&rom).unwrap();
    core_prg.write_cpu_bus(0x8000, 0x10);
    let hash_prg = core_prg.state_hash();

    let mut core_chr = NesCore::new();
    core_chr.load_ines_rom(&rom).unwrap();
    core_chr.write_cpu_bus(0x8000, 0x01);
    let hash_chr = core_chr.state_hash();

    let mut core_both = NesCore::new();
    core_both.load_ines_rom(&rom).unwrap();
    core_both.write_cpu_bus(0x8000, 0x11);
    let hash_both = core_both.state_hash();

    assert_eq!(
        hash_both ^ hash_base,
        (hash_prg ^ hash_base) ^ (hash_chr ^ hash_base),
        "GXROM fields must be combined with XOR, not OR/AND"
    );
}

#[test]
fn axrom_hash_component_combines_fields_with_xor() {
    let mut core1 = NesCore::new();
    let rom = load_rom(&mut core1, 7, 4, 0);

    core1.write_cpu_bus(0x8000, 0x11); // sets bank=1, nametable=1
    let hash_both = core1.state_hash();

    let mut core2 = NesCore::new();
    core2.load_ines_rom(&rom).unwrap();
    core2.write_cpu_bus(0x8000, 0x01); // sets bank=1, nametable=0
    let hash_bank = core2.state_hash();

    let mut core3 = NesCore::new();
    core3.load_ines_rom(&rom).unwrap();
    core3.write_cpu_bus(0x8000, 0x10); // sets bank=0, nametable=1
    let hash_nt = core3.state_hash();

    let mut core_base = NesCore::new();
    core_base.load_ines_rom(&rom).unwrap();
    let hash_base = core_base.state_hash();

    assert_eq!(
        hash_both ^ hash_base,
        (hash_bank ^ hash_base) ^ (hash_nt ^ hash_base),
        "Fields must be combined with XOR, not OR/AND"
    );
}

#[test]
fn mmc3_hash_component_combines_fields_with_xor() {
    let mut core1 = NesCore::new();
    // 4 PRG banks
    let mut rom = vec![0_u8; 16 + 4 * 16 * 1024];
    rom[0] = 0x4E;
    rom[1] = 0x45;
    rom[2] = 0x53;
    rom[3] = 0x1A;
    rom[4] = 4;
    rom[5] = 0;
    rom[6] = (4 & 0x0F) << 4;
    rom[7] = 4 & 0xF0;
    for (i, byte) in rom.iter_mut().enumerate().skip(16) {
        *byte = (i % 256) as u8;
    }

    core1.load_ines_rom(&rom).unwrap();
    let hash_base = core1.state_hash();

    let mut core_a = NesCore::new();
    core_a.load_ines_rom(&rom).unwrap();
    core_a.write_cpu_bus(0x8000, 6);
    core_a.write_cpu_bus(0x8001, 1);
    let _ = core_a.read_memory(0x8000);
    let hash_a = core_a.state_hash();

    let mut core_b = NesCore::new();
    core_b.load_ines_rom(&rom).unwrap();
    core_b.write_cpu_bus(0x8000, 7);
    core_b.write_cpu_bus(0x8001, 2);
    let _ = core_b.read_memory(0x8000);
    let hash_b = core_b.state_hash();

    let mut core_both = NesCore::new();
    core_both.load_ines_rom(&rom).unwrap();
    core_both.write_cpu_bus(0x8000, 6);
    core_both.write_cpu_bus(0x8001, 1);
    core_both.write_cpu_bus(0x8000, 7);
    core_both.write_cpu_bus(0x8001, 2);
    let _ = core_both.read_memory(0x8000);
    let hash_both = core_both.state_hash();

    assert_eq!(
        hash_both ^ hash_base,
        (hash_a ^ hash_base) ^ (hash_b ^ hash_base),
        "MMC3 fields must be combined with XOR, not OR/AND"
    );
}

#[test]
fn mmc3_hash_component_fields_shift_and_combine_correctly() {
    let mut core_base = NesCore::new();
    let mut rom = vec![0_u8; 16 + 4 * 16 * 1024];
    rom[0] = 0x4E;
    rom[1] = 0x45;
    rom[2] = 0x53;
    rom[3] = 0x1A;
    rom[4] = 4;
    rom[5] = 0;
    rom[6] = (4 & 0x0F) << 4;
    rom[7] = 4 & 0xF0;

    for i in 0..8 {
        for j in 0..8192 {
            rom[16 + (i * 8192) + j] = i as u8;
        }
    }

    core_base.load_ines_rom(&rom).unwrap();
    let hash_base = core_base.state_hash();

    let mut core_1 = NesCore::new();
    core_1.load_ines_rom(&rom).unwrap();
    let val_base_8000 = core_1.read_memory(0x8000);
    core_1.write_cpu_bus(0x8000, 6);
    core_1.write_cpu_bus(0x8001, 1);
    let val_new_8000 = core_1.read_memory(0x8000);

    // In mmc3 hash component: `(u64::from(mapper.read_prg(0x8000)) << 8)`
    // The change from base to core_1 is `((new << 8) ^ (old << 8))` rotated by 53
    let expected_diff_8000 = ((val_new_8000 as u64) << 8) ^ ((val_base_8000 as u64) << 8);
    assert_eq!(
        core_1.state_hash() ^ hash_base,
        expected_diff_8000.rotate_left(53),
        "MMC3 PRG at 0x8000 must strictly shift its read byte by 8"
    );

    let mut core_2 = NesCore::new();
    core_2.load_ines_rom(&rom).unwrap();
    let val_base_a000 = core_2.read_memory(0xA000);
    core_2.write_cpu_bus(0x8000, 7);
    core_2.write_cpu_bus(0x8001, 2);
    let val_new_a000 = core_2.read_memory(0xA000);

    // In mmc3 hash component: `(u64::from(mapper.read_prg(0xA000)) << 16)`
    let expected_diff_a000 = ((val_new_a000 as u64) << 16) ^ ((val_base_a000 as u64) << 16);
    assert_eq!(
        core_2.state_hash() ^ hash_base,
        expected_diff_a000.rotate_left(53),
        "MMC3 PRG at 0xA000 must strictly shift its read byte by 16"
    );
}
