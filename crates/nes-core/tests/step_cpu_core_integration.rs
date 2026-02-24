use nes_core::cpu::CpuError;
use nes_core::{Command, CoreError, NesCore};

#[test]
fn step_cpu_executes_instruction_and_tracks_trace() {
    let mut core = NesCore::new();
    core.load_cpu_bytes(0xC000, &[0xA9, 0x01, 0xAA]);

    core.execute(Command::StepCpu).unwrap();

    assert_eq!(core.cpu_a(), 0x01);
    assert_eq!(core.cpu_x(), 0x00);
    assert_eq!(core.cpu_pc(), 0xC002);
    assert_eq!(core.total_cycles(), 1);
    assert_eq!(
        core.last_cpu_trace(),
        Some("C000  A9 01     LDA #$01                        A:00 X:00 Y:00 P:24 SP:FD")
    );

    core.execute(Command::StepCpu).unwrap();

    assert_eq!(core.cpu_x(), 0x01);
    assert_eq!(core.cpu_pc(), 0xC003);
    assert_eq!(core.total_cycles(), 2);
    assert_eq!(
        core.last_cpu_trace(),
        Some("C002  AA        TAX                             A:01 X:00 Y:00 P:24 SP:FD")
    );
}

#[test]
fn step_cpu_surfaces_unknown_opcode_errors() {
    let mut core = NesCore::new();
    core.load_cpu_bytes(0xC000, &[0x02]);

    let err = core.execute(Command::StepCpu).unwrap_err();

    assert_eq!(err, CoreError::CpuStepFailed(CpuError::UnknownOpcode(0x02)));
}
