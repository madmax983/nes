use nes_core::{Command, NesCore};

#[test]
fn step_frame_advances_monotonic_cycle_counter() {
    let mut core = NesCore::new();
    let c0 = core.total_cycles();
    core.execute(Command::StepFrame).unwrap();
    let c1 = core.total_cycles();
    assert!(c1 > c0);
}

#[test]
fn identical_command_sequences_yield_identical_state_hash() {
    let mut a = NesCore::new();
    let mut b = NesCore::new();

    let nops = [0xEA; 10];
    a.load_cpu_bytes(0xC000, &nops);
    b.load_cpu_bytes(0xC000, &nops);

    for _ in 0..10 {
        a.execute(Command::StepCpu).unwrap();
        b.execute(Command::StepCpu).unwrap();
    }
    assert_eq!(a.state_hash(), b.state_hash());
}
