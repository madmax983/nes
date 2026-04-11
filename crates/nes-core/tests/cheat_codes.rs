use nes_core::{CheatCode, NesCore};

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
fn cheat_code_decodes_six_letter_codes() {
    let code: CheatCode = "GOSSIP".parse().expect("valid six-letter code");
    assert_eq!(code.address(), 0xD1DD);
    assert_eq!(code.value(), 0x14);
    assert_eq!(code.compare(), None);
}

#[test]
fn cheat_code_decodes_eight_letter_codes_with_compare() {
    let code: CheatCode = "ZEXPYGLA".parse().expect("valid eight-letter code");
    assert_eq!(code.address(), 0x94A7);
    assert_eq!(code.value(), 0x02);
    assert_eq!(code.compare(), Some(0x03));
}

#[test]
fn cheat_code_rejects_invalid_codes() {
    let short = "ABC"
        .parse::<CheatCode>()
        .expect_err("short code should fail");
    assert!(
        short.to_string().contains("6 or 8"),
        "unexpected error: {short}"
    );

    let invalid_letter = "GOSS1P"
        .parse::<CheatCode>()
        .expect_err("unexpected alphabet should fail");
    assert!(
        invalid_letter
            .to_string()
            .contains("invalid cheat code character"),
        "unexpected error: {invalid_letter}"
    );
}

#[test]
fn cheat_codes_override_nrom_reads_and_can_be_cleared() {
    let mut rom = sample_ines(0, 1);
    let prg_start = 16;
    let cpu_addr = 0xD1DD;
    let prg_offset = usize::from(cpu_addr - 0xC000);
    rom[prg_start + prg_offset] = 0xEA;

    let mut core = NesCore::new();
    core.load_ines_rom(&rom).expect("sample rom should load");

    assert_eq!(core.read_memory(cpu_addr), 0xEA);

    core.add_cheat_code("GOSSIP")
        .expect("valid cheat code should apply");
    assert_eq!(core.read_memory(cpu_addr), 0x14);

    core.clear_cheat_codes();
    assert_eq!(core.read_memory(cpu_addr), 0xEA);
}

#[test]
fn compare_guarded_cheat_codes_track_uxrom_bank_switches() {
    let mut rom = sample_ines(2, 3);
    let prg_start = 16;
    let bank_size = 16 * 1024;
    let cpu_addr = 0x94A7;
    let prg_offset = usize::from(cpu_addr - 0x8000);

    rom[prg_start + prg_offset] = 0x03;
    rom[prg_start + bank_size + prg_offset] = 0x09;

    let mut core = NesCore::new();
    core.load_ines_rom(&rom).expect("sample rom should load");
    assert_eq!(core.read_memory(cpu_addr), 0x03);

    core.add_cheat_code("ZEXPYGLA")
        .expect("compare-guarded code should apply");
    assert_eq!(core.read_memory(cpu_addr), 0x02);

    core.write_cpu_bus(0x8000, 0x01);
    assert_eq!(
        core.read_memory(cpu_addr),
        0x09,
        "compare-guarded code should stop applying when the bank byte changes"
    );

    core.write_cpu_bus(0x8000, 0x00);
    assert_eq!(core.read_memory(cpu_addr), 0x02);
}

#[test]
fn state_hash_differentiates_cheat_code_variations() {
    let empty_core = NesCore::new();
    let hash_empty = empty_core.state_hash();

    let mut core1 = NesCore::new();
    core1.add_cheat_code("GOSSIP").unwrap();
    let hash1 = core1.state_hash();

    let mut core2 = NesCore::new();
    core2.add_cheat_code("SXTPOU").unwrap();
    let hash2 = core2.state_hash();

    let mut core3 = NesCore::new();
    core3.add_cheat_code("ZEXPYGLA").unwrap();
    let hash3 = core3.state_hash();

    // Different order of same codes should produce different hash
    let mut core4 = NesCore::new();
    core4.add_cheat_code("GOSSIP").unwrap();
    core4.add_cheat_code("SXTPOU").unwrap();
    let hash4 = core4.state_hash();

    let mut core5 = NesCore::new();
    core5.add_cheat_code("SXTPOU").unwrap();
    core5.add_cheat_code("GOSSIP").unwrap();
    let hash5 = core5.state_hash();

    // Verify all hashes are unique
    let mut hashes = vec![hash_empty, hash1, hash2, hash3, hash4, hash5];
    hashes.sort();
    hashes.dedup();
    assert_eq!(
        hashes.len(),
        6,
        "All cheat code variations should produce unique state hashes"
    );
}
