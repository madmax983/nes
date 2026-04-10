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
fn cheat_code_hash_component_is_shift_sensitive_via_modulo_division() {
    let mut core2 = NesCore::new();
    for _ in 0..64 {
        core2.add_cheat_code("GOSSIP").unwrap();
    }
    let core2_hash = core2.state_hash();

    let mut core3 = NesCore::new();
    for _ in 0..65 {
        core3.add_cheat_code("GOSSIP").unwrap();
    }
    let core3_hash = core3.state_hash();

    assert_ne!(core2_hash, core3_hash);

    let empty = NesCore::new().state_hash();
    let mut c1 = NesCore::new();
    c1.add_cheat_code("GOSSIP").unwrap();
    let val = c1.state_hash() ^ empty;

    let _expected_component = 0x000000001FF141DD_u64;
    assert_eq!(val, 575426418750521344);
}
