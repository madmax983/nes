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

    for cmd in &log {
        a.execute(*cmd).unwrap();
    }
    let target = a.state_hash();

    let mut b = NesCore::new();
    b.replay(&log).unwrap();
    assert_eq!(target, b.state_hash());
}
