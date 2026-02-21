use nes_core::cpu::{Cpu, CpuError};

#[test]
fn cpu_executes_lda_tax_inx_sequence() {
    let mut cpu = Cpu::new(0xC000);
    cpu.load_bytes(0xC000, &[0xA9, 0x01, 0xAA, 0xE8]);

    cpu.step_with_trace().unwrap();
    assert_eq!(cpu.a(), 0x01);
    assert_eq!(cpu.x(), 0x00);
    assert_eq!(cpu.pc(), 0xC002);

    cpu.step_with_trace().unwrap();
    assert_eq!(cpu.x(), 0x01);
    assert_eq!(cpu.pc(), 0xC003);

    cpu.step_with_trace().unwrap();
    assert_eq!(cpu.x(), 0x02);
    assert_eq!(cpu.pc(), 0xC004);
}

#[test]
fn nestest_style_trace_for_lda_immediate_matches_expected_prefix() {
    let mut cpu = Cpu::new(0xC000);
    cpu.load_bytes(0xC000, &[0xA9, 0x01]);

    let trace = cpu.step_with_trace().unwrap();

    assert_eq!(
        trace,
        "C000  A9 01     LDA #$01                        A:00 X:00 Y:00 P:24 SP:FD"
    );
}

#[test]
fn unknown_opcode_returns_error() {
    let mut cpu = Cpu::new(0x8000);
    cpu.load_bytes(0x8000, &[0xFF]);

    let err = cpu.step_with_trace().unwrap_err();
    assert_eq!(err, CpuError::UnknownOpcode(0xFF));
}

#[test]
fn nop_advances_pc_and_preserves_registers() {
    let mut cpu = Cpu::new(0x9000);
    cpu.load_bytes(0x9000, &[0xEA]);

    cpu.step_with_trace().unwrap();

    assert_eq!(cpu.pc(), 0x9001);
    assert_eq!(cpu.a(), 0x00);
    assert_eq!(cpu.x(), 0x00);
}
