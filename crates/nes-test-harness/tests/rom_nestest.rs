use std::fs;

use nes_core::{Command, NesCore};

mod support;

#[test]
#[ignore = "requires roms.nestest in nes.toml"]
fn nestest_boot_sequence_matches_expected_prefix() {
    let rom_path = support::nestest_rom_path();

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
    let expected_cycles = [2_u64, 4, 6, 8, 12, 15];

    for (step, expected_prefix) in expected.iter().enumerate() {
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
        assert_eq!(
            core.total_cycles(),
            expected_cycles[step],
            "unexpected cycle total after trace {expected_prefix}",
        );
    }
}

#[test]
#[ignore = "requires roms.nestest in nes.toml"]
fn nestest_runs_instruction_window_without_unknown_opcode() {
    let rom_path = support::nestest_rom_path();

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
