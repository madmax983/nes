use nes_core::{Button, Command, NesCore};

#[test]
fn recorded_command_log_replays_to_identical_state_hash() {
    let mut a = NesCore::new();
    let log = vec![
        Command::StepCpu,
        Command::Pause,
        Command::Resume,
        Command::PressButton(Button::A),
        Command::StepFrame,
    ];

    a.load_cpu_bytes(0xC000, &[0xEA]);
    for cmd in &log {
        a.execute(*cmd).expect("Should succeed");
    }
    let target = a.state_hash();

    let mut b = NesCore::new();
    b.load_cpu_bytes(0xC000, &[0xEA]);
    b.replay(&log).expect("Should succeed");
    assert_eq!(target, b.state_hash());
}
