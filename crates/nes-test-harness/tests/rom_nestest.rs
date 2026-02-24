use std::env;
use std::fs;
use std::path::Path;

use nes_core::{Command, NesCore};

const DEFAULT_NESTEST_ROM: &str = r"C:\Users\markm\roms\nestest.nes";

fn resolve_nestest_rom_path() -> String {
    match env::var("NESTEST_ROM_PATH") {
        Ok(path) if !path.trim().is_empty() => path,
        _ => DEFAULT_NESTEST_ROM.to_owned(),
    }
}

#[test]
#[ignore = "requires NESTEST_ROM_PATH or default local ROM path"]
fn nestest_boot_sequence_matches_expected_prefix() {
    let rom_path = resolve_nestest_rom_path();
    assert!(
        Path::new(&rom_path).exists(),
        "NESTEST ROM path does not exist: {rom_path}"
    );

    let bytes = fs::read(&rom_path).expect("failed to read nestest ROM");
    let mut core = NesCore::new();
    let info = core
        .load_ines_rom(&bytes)
        .expect("failed to load nestest ROM");
    assert_eq!(info.mapper_id, 0, "nestest should be mapper 0");
    assert_eq!(info.reset_pc, 0xC004, "nestest reset vector mismatch");

    let expected = [
        "C004  78        SEI",
        "C005  D8        CLD",
        "C006  A2 FF     LDX #$FF",
        "C008  9A        TXS",
        "C009  AD 02 20  LDA $2002",
        "C00C  10 FB     BPL $C009",
    ];

    for expected_prefix in expected {
        core.execute(Command::StepCpu).unwrap_or_else(|err| {
            panic!(
                "nestest prefix step failed at pc={:04X}: {err}",
                core.cpu_pc()
            )
        });
        let trace = core
            .last_cpu_trace()
            .expect("missing cpu trace after StepCpu");
        assert!(
            trace.starts_with(expected_prefix),
            "trace prefix mismatch\nexpected: {expected_prefix}\nactual:   {trace}"
        );
    }
}

#[test]
#[ignore = "requires NESTEST_ROM_PATH or default local ROM path"]
fn nestest_runs_instruction_window_without_unknown_opcode() {
    let rom_path = resolve_nestest_rom_path();
    assert!(
        Path::new(&rom_path).exists(),
        "NESTEST ROM path does not exist: {rom_path}"
    );

    let bytes = fs::read(&rom_path).expect("failed to read nestest ROM");
    let mut core = NesCore::new();
    core.load_ines_rom(&bytes)
        .expect("failed to load nestest ROM");

    for step in 0..50_000_u32 {
        core.execute(Command::StepCpu).unwrap_or_else(|err| {
            panic!(
                "nestest failed at step {step}, pc={:04X}: {err}",
                core.cpu_pc()
            )
        });
    }
}
