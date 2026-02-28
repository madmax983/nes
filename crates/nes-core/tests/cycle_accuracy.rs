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
fn step_frame_advances_to_next_ppu_frame() {
    let mut core = NesCore::new();
    core.load_cpu_bytes(0xC000, &[0xEA, 0x4C, 0x00, 0xC0]); // NOP ; JMP $C000

    let before_cpu = core.total_cycles();
    let before_ppu = core.ppu_total_cycles();
    let before_frame = core.ppu_frame_counter();
    core.execute(Command::StepFrame).unwrap();

    let consumed_cpu = core.total_cycles() - before_cpu;
    let consumed_ppu = core.ppu_total_cycles() - before_ppu;
    assert_eq!(core.ppu_frame_counter(), before_frame + 1);
    assert!(consumed_cpu > 0);
    assert!(consumed_ppu >= 341 * 262);
    assert!(consumed_ppu <= 341 * 262 + 9);
}
