mod support;

use nes_ai::actions::ControlAction;
use support::mock_profile::make_mock_env;

#[test]
fn reset_and_replayed_action_sequence_are_deterministic() {
    let mut a = make_mock_env();
    let mut b = make_mock_env();

    let _ = a.reset().unwrap();
    let _ = b.reset().unwrap();

    for action in [
        ControlAction::Right,
        ControlAction::RightA,
        ControlAction::Noop,
        ControlAction::RightB,
    ] {
        a.step(action).unwrap();
        b.step(action).unwrap();
    }

    assert_eq!(a.core().state_hash(), b.core().state_hash());
}
