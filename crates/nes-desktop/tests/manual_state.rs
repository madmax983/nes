use std::time::{SystemTime, UNIX_EPOCH};

use nes_core::{Command, NesCore};
use nes_desktop::manual_state::{load_state_file, quicksave_path_for_rom, save_state_file};
use nes_desktop::rta::compute_rom_hash;

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

fn unique_temp_dir() -> std::path::PathBuf {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should be after unix epoch")
        .as_nanos();
    std::env::temp_dir().join(format!("nes-desktop-manual-state-{stamp}"))
}

#[test]
fn quicksave_path_uses_sanitized_rom_stem_and_hash_prefix() {
    let path = quicksave_path_for_rom(
        std::path::Path::new(r"C:\roms\Super Mario Bros. (World).nes"),
        "0123456789abcdef",
    );
    assert_eq!(
        path,
        std::path::PathBuf::from("savestates")
            .join("Super_Mario_Bros___World_-01234567.state.json")
    );
}

#[test]
fn save_state_file_round_trips_snapshot_for_loaded_rom() {
    let rom = sample_ines(0, 2);
    let rom_hash = compute_rom_hash(&rom);
    let mut core = NesCore::new();
    core.load_ines_rom(&rom).expect("sample rom should load");
    core.execute(Command::StepFrame)
        .expect("sample rom should step one frame");
    let snapshot = core.save_state();

    let temp_dir = unique_temp_dir();
    let state_path = temp_dir.join("slot0.state.json");
    save_state_file(&state_path, &rom_hash, &snapshot).expect("save state should serialize");

    let restored = load_state_file(&state_path, &rom_hash).expect("state should deserialize");
    assert_eq!(restored, snapshot);

    let _ = std::fs::remove_dir_all(temp_dir);
}

#[test]
fn load_state_file_rejects_hash_mismatch() {
    let rom = sample_ines(0, 2);
    let rom_hash = compute_rom_hash(&rom);
    let mut core = NesCore::new();
    core.load_ines_rom(&rom).expect("sample rom should load");
    let snapshot = core.save_state();

    let temp_dir = unique_temp_dir();
    let state_path = temp_dir.join("slot0.state.json");
    save_state_file(&state_path, &rom_hash, &snapshot).expect("save state should serialize");

    let err = load_state_file(&state_path, "deadbeef").expect_err("wrong rom hash must fail");
    assert!(err.contains("ROM hash mismatch"));

    let _ = std::fs::remove_dir_all(temp_dir);
}
