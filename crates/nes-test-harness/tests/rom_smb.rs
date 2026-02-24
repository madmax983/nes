use std::env;
use std::fs;
use std::path::Path;

use nes_core::{Command, NesCore};

#[test]
#[ignore = "requires SMB_ROM_PATH"]
fn smb_rom_loads_and_exposes_reset_vector() {
    let rom_path = env::var("SMB_ROM_PATH").expect("set SMB_ROM_PATH to run this test");
    assert!(!rom_path.trim().is_empty(), "SMB_ROM_PATH cannot be empty");
    assert!(
        Path::new(&rom_path).exists(),
        "SMB ROM path does not exist: {rom_path}"
    );

    let bytes = fs::read(&rom_path).expect("failed to read SMB ROM");
    let mut core = NesCore::new();
    let info = core.load_ines_rom(&bytes).expect("failed to load SMB ROM");
    assert!(
        info.reset_pc >= 0x8000,
        "unexpected reset vector {:04X}",
        info.reset_pc
    );
}

#[test]
#[ignore = "requires SMB_ROM_PATH"]
fn smb_boot_window_runs_without_unknown_opcode() {
    let rom_path = env::var("SMB_ROM_PATH").expect("set SMB_ROM_PATH to run this test");
    assert!(!rom_path.trim().is_empty(), "SMB_ROM_PATH cannot be empty");
    assert!(
        Path::new(&rom_path).exists(),
        "SMB ROM path does not exist: {rom_path}"
    );

    let bytes = fs::read(&rom_path).expect("failed to read SMB ROM");
    let mut core = NesCore::new();
    core.load_ines_rom(&bytes).expect("failed to load SMB ROM");

    for step in 0..20_000_u32 {
        core.execute(Command::StepCpu).unwrap_or_else(|err| {
            panic!(
                "SMB boot window failed at step {step}, pc={:04X}: {err}",
                core.cpu_pc()
            )
        });
    }
}
