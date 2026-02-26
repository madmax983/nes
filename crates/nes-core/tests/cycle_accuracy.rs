use nes_core::{Command, NesCore};

#[test]
fn branch_taken_and_page_cross_add_cycles() {
    let mut core = NesCore::new();
    core.load_cpu_bytes(
        0xC000,
        &[
            0xA9, 0x00, // LDA #$00 (2 cycles, sets Z)
            0xF0, 0x80, // BEQ $BF84 (base 2 + taken 1 + page-cross 1)
        ],
    );

    core.execute(Command::StepCpu).unwrap();
    assert_eq!(core.total_cycles(), 2);

    core.execute(Command::StepCpu).unwrap();
    assert_eq!(core.cpu_pc(), 0xBF84);
    assert_eq!(core.total_cycles(), 6);
}

#[test]
fn absolute_x_page_cross_adds_cycle_for_reads() {
    let mut core = NesCore::new();
    core.load_cpu_bytes(
        0xC000,
        &[
            0xA2, 0x01, // LDX #$01 (2 cycles)
            0xBD, 0xFF, 0x00, // LDA $00FF,X -> $0100 (4 + 1 cycles)
        ],
    );
    core.write_cpu_bus(0x0100, 0x42);

    core.execute(Command::StepCpu).unwrap();
    assert_eq!(core.total_cycles(), 2);

    core.execute(Command::StepCpu).unwrap();
    assert_eq!(core.cpu_a(), 0x42);
    assert_eq!(core.total_cycles(), 7);
}

#[test]
fn step_frame_uses_instruction_cycles_budget() {
    let mut core = NesCore::new();
    let before = core.total_cycles();

    core.execute(Command::StepFrame).unwrap();

    let consumed = core.total_cycles() - before;
    assert!(consumed >= 29_780);
    assert!(consumed <= 29_787);
}
