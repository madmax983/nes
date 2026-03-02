use nes_core::cpu::{Cpu, CpuBusAccess, CpuBusAccessKind, CpuError, CpuPrgWrite};

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
fn cpu_executes_ldx_ldy_txa_tya_sequence() {
    let mut cpu = Cpu::new(0xC000);
    cpu.load_bytes(0xC000, &[0xA2, 0x10, 0xA0, 0x20, 0x8A, 0x98]);

    cpu.step_with_trace().unwrap();
    assert_eq!(cpu.x(), 0x10);
    assert_eq!(cpu.pc(), 0xC002);

    cpu.step_with_trace().unwrap();
    assert_eq!(cpu.y(), 0x20);
    assert_eq!(cpu.pc(), 0xC004);

    cpu.step_with_trace().unwrap();
    assert_eq!(cpu.a(), 0x10);
    assert_eq!(cpu.pc(), 0xC005);

    cpu.step_with_trace().unwrap();
    assert_eq!(cpu.a(), 0x20);
    assert_eq!(cpu.pc(), 0xC006);
}

#[test]
fn jsr_and_rts_round_trip_through_stack() {
    let mut cpu = Cpu::new(0xC000);
    cpu.load_bytes(0xC000, &[0x20, 0x05, 0xC0, 0xEA, 0xEA, 0xE8, 0x60]);

    let jsr_trace = cpu.step_with_trace().unwrap();
    assert_eq!(cpu.pc(), 0xC005);
    assert_eq!(cpu.sp(), 0xFB);
    assert_eq!(cpu.read_byte(0x01FD), 0xC0);
    assert_eq!(cpu.read_byte(0x01FC), 0x02);
    assert_eq!(
        jsr_trace,
        "C000  20 05 C0  JSR $C005                       A:00 X:00 Y:00 P:24 SP:FD"
    );

    cpu.step_with_trace().unwrap();
    assert_eq!(cpu.x(), 0x01);
    assert_eq!(cpu.pc(), 0xC006);

    let rts_trace = cpu.step_with_trace().unwrap();
    assert_eq!(cpu.pc(), 0xC003);
    assert_eq!(cpu.sp(), 0xFD);
    assert_eq!(
        rts_trace,
        "C006  60        RTS                             A:00 X:01 Y:00 P:24 SP:FB"
    );

    cpu.step_with_trace().unwrap();
    assert_eq!(cpu.pc(), 0xC004);
}

#[test]
fn beq_taken_when_zero_flag_is_set() {
    let mut cpu = Cpu::new(0xC000);
    cpu.load_bytes(
        0xC000,
        &[0xA9, 0x01, 0xC9, 0x01, 0xF0, 0x02, 0xA2, 0x00, 0xA2, 0x7F],
    );

    cpu.step_with_trace().unwrap();
    cpu.step_with_trace().unwrap();

    let branch_trace = cpu.step_with_trace().unwrap();
    assert_eq!(cpu.pc(), 0xC008);
    assert_eq!(
        branch_trace,
        "C004  F0 02     BEQ $C008                       A:01 X:00 Y:00 P:27 SP:FD"
    );

    cpu.step_with_trace().unwrap();
    assert_eq!(cpu.x(), 0x7F);
    assert_eq!(cpu.pc(), 0xC00A);
}

#[test]
fn bne_taken_when_zero_flag_is_clear() {
    let mut cpu = Cpu::new(0xC000);
    cpu.load_bytes(
        0xC000,
        &[0xA9, 0x01, 0xC9, 0x02, 0xD0, 0x02, 0xA2, 0x00, 0xA2, 0x55],
    );

    cpu.step_with_trace().unwrap();
    cpu.step_with_trace().unwrap();

    let branch_trace = cpu.step_with_trace().unwrap();
    assert_eq!(cpu.pc(), 0xC008);
    assert_eq!(
        branch_trace,
        "C004  D0 02     BNE $C008                       A:01 X:00 Y:00 P:A4 SP:FD"
    );

    cpu.step_with_trace().unwrap();
    assert_eq!(cpu.x(), 0x55);
    assert_eq!(cpu.pc(), 0xC00A);
}

#[test]
fn beq_not_taken_when_zero_flag_is_clear() {
    let mut cpu = Cpu::new(0xC000);
    cpu.load_bytes(
        0xC000,
        &[0xA9, 0x01, 0xC9, 0x02, 0xF0, 0x02, 0xA2, 0x33, 0xA2, 0x55],
    );

    cpu.step_with_trace().unwrap();
    cpu.step_with_trace().unwrap();

    cpu.step_with_trace().unwrap();
    assert_eq!(cpu.pc(), 0xC006);

    cpu.step_with_trace().unwrap();
    assert_eq!(cpu.x(), 0x33);
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
fn nestest_style_trace_for_ldx_immediate_matches_expected_prefix() {
    let mut cpu = Cpu::new(0xC000);
    cpu.load_bytes(0xC000, &[0xA2, 0x10]);

    let trace = cpu.step_with_trace().unwrap();

    assert_eq!(
        trace,
        "C000  A2 10     LDX #$10                        A:00 X:00 Y:00 P:24 SP:FD"
    );
}

#[test]
fn unknown_opcode_returns_error() {
    let mut cpu = Cpu::new(0x8000);
    cpu.load_bytes(0x8000, &[0x02]);

    let err = cpu.step_with_trace().unwrap_err();
    assert_eq!(err, CpuError::UnknownOpcode(0x02));
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

#[test]
fn sta_absolute_writes_memory_and_emits_prg_write() {
    let mut cpu = Cpu::new(0xC000);
    cpu.load_bytes(0xC000, &[0xA9, 0x05, 0x8D, 0x00, 0x80]);

    cpu.step_with_trace().unwrap();

    let trace = cpu.step_with_trace().unwrap();

    assert_eq!(cpu.read_byte(0x8000), 0x05);
    assert_eq!(cpu.pc(), 0xC005);
    assert_eq!(
        trace,
        "C002  8D 00 80  STA $8000                       A:05 X:00 Y:00 P:24 SP:FD"
    );

    let mut prg_writes = Vec::new();
    cpu.swap_prg_writes(&mut prg_writes);
    assert_eq!(
        prg_writes,
        vec![CpuPrgWrite {
            addr: 0x8000,
            value: 0x05
        }]
    );
}

#[test]
fn txs_and_stx_absolute_update_stack_and_memory() {
    let mut cpu = Cpu::new(0xC000);
    cpu.load_bytes(0xC000, &[0xA2, 0xAB, 0x9A, 0x8E, 0x34, 0x12]);

    cpu.step_with_trace().unwrap();
    cpu.step_with_trace().unwrap();
    cpu.step_with_trace().unwrap();

    assert_eq!(cpu.sp(), 0xAB);
    assert_eq!(cpu.read_byte(0x1234), 0xAB);
    assert_eq!(cpu.pc(), 0xC006);
}

#[test]
fn bpl_and_bmi_follow_negative_flag() {
    let mut cpu = Cpu::new(0xC000);
    cpu.load_bytes(
        0xC000,
        &[
            0xA9, 0x01, // LDA #$01 (N clear)
            0x10, 0x02, // BPL +2 (taken)
            0xA2, 0x00, // skipped
            0xA2, 0x7F, // executed
            0xA9, 0x80, // LDA #$80 (N set)
            0x30, 0x02, // BMI +2 (taken)
            0xA2, 0x01, // skipped
            0xA2, 0x55, // executed
        ],
    );

    cpu.step_with_trace().unwrap();
    cpu.step_with_trace().unwrap();
    cpu.step_with_trace().unwrap();
    assert_eq!(cpu.x(), 0x7F);

    cpu.step_with_trace().unwrap();
    cpu.step_with_trace().unwrap();
    cpu.step_with_trace().unwrap();
    assert_eq!(cpu.x(), 0x55);
}

#[test]
fn sta_absolute_x_writes_indexed_target() {
    let mut cpu = Cpu::new(0xC000);
    cpu.load_bytes(0xC000, &[0xA9, 0x77, 0xA2, 0x01, 0x9D, 0x00, 0x80]);

    cpu.step_with_trace().unwrap();
    cpu.step_with_trace().unwrap();
    cpu.step_with_trace().unwrap();

    assert_eq!(cpu.read_byte(0x8001), 0x77);
    let mut prg_writes = Vec::new();
    cpu.swap_prg_writes(&mut prg_writes);
    assert_eq!(
        prg_writes,
        vec![CpuPrgWrite {
            addr: 0x8001,
            value: 0x77
        }]
    );
}

#[test]
fn bus_microphase_for_lda_immediate_matches_cycle_reads() {
    let mut cpu = Cpu::new(0xC000);
    cpu.load_bytes(0xC000, &[0xA9, 0x42]);

    cpu.step_with_trace().unwrap();
    let mut bus = Vec::new();
    cpu.swap_bus_trace(&mut bus);

    assert_eq!(
        bus,
        vec![
            CpuBusAccess {
                addr: 0xC000,
                value: 0xA9,
                kind: CpuBusAccessKind::Read
            },
            CpuBusAccess {
                addr: 0xC001,
                value: 0x42,
                kind: CpuBusAccessKind::Read
            },
        ]
    );
}

#[test]
fn bus_microphase_for_sta_absolute_matches_fetch_fetch_fetch_write() {
    let mut cpu = Cpu::new(0xC000);
    cpu.load_bytes(0xC000, &[0xA9, 0x77, 0x8D, 0x00, 0x80]);
    cpu.step_with_trace().unwrap();

    cpu.step_with_trace().unwrap();
    let mut bus = Vec::new();
    cpu.swap_bus_trace(&mut bus);

    assert_eq!(
        bus,
        vec![
            CpuBusAccess {
                addr: 0xC002,
                value: 0x8D,
                kind: CpuBusAccessKind::Read
            },
            CpuBusAccess {
                addr: 0xC003,
                value: 0x00,
                kind: CpuBusAccessKind::Read
            },
            CpuBusAccess {
                addr: 0xC004,
                value: 0x80,
                kind: CpuBusAccessKind::Read
            },
            CpuBusAccess {
                addr: 0x8000,
                value: 0x77,
                kind: CpuBusAccessKind::Write
            },
        ]
    );
}

#[test]
fn irq_service_vectors_when_interrupts_enabled() {
    let mut cpu = Cpu::new(0xC000);
    cpu.load_bytes(0xC000, &[0x58]); // CLI
    cpu.load_bytes(0xFFFE, &[0x34, 0x12]);

    cpu.step_with_trace().unwrap();
    let mut dummy = Vec::new();
    cpu.swap_bus_trace(&mut dummy);

    assert!(cpu.service_irq());
    assert_eq!(cpu.pc(), 0x1234);
    assert_eq!(cpu.sp(), 0xFA);
    assert_eq!(cpu.read_byte(0x01FD), 0xC0);
    assert_eq!(cpu.read_byte(0x01FC), 0xC001_u16 as u8);

    let mut bus = Vec::new();
    cpu.swap_bus_trace(&mut bus);
    assert_eq!(bus.len(), 5);
    assert_eq!(bus[0].addr, 0x01FD);
    assert_eq!(bus[0].kind, CpuBusAccessKind::Write);
    assert_eq!(bus[1].addr, 0x01FC);
    assert_eq!(bus[1].kind, CpuBusAccessKind::Write);
    assert_eq!(bus[2].addr, 0x01FB);
    assert_eq!(bus[2].kind, CpuBusAccessKind::Write);
    assert_eq!(bus[3].addr, 0xFFFE);
    assert_eq!(bus[3].kind, CpuBusAccessKind::Read);
    assert_eq!(bus[4].addr, 0xFFFF);
    assert_eq!(bus[4].kind, CpuBusAccessKind::Read);
}

#[test]
fn irq_service_is_masked_when_interrupt_disable_is_set() {
    let mut cpu = Cpu::new(0xC000);
    cpu.load_bytes(0xFFFE, &[0x34, 0x12]);

    assert!(!cpu.service_irq());
    assert_eq!(cpu.pc(), 0xC000);
    assert_eq!(cpu.sp(), 0xFD);
    let mut bus = Vec::new();
    cpu.swap_bus_trace(&mut bus);
    assert!(bus.is_empty());
}
