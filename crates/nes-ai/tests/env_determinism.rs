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

#[test]
fn env_exposes_tensor_ready_observation_and_stops_at_frame_budget() {
    let mut env = make_mock_env();

    let _ = env.reset().unwrap();
    let observation = env.observation().unwrap();

    assert_eq!(observation.frame_stack, 4);
    assert_eq!(observation.width, 84);
    assert_eq!(observation.height, 84);
    assert_eq!(observation.frames.len(), 4 * 84 * 84);
    assert_eq!(observation.features.len(), 6);

    let mut done = false;
    for _ in 0..60 {
        let step = env.step(ControlAction::Noop).unwrap();
        if step.done {
            done = true;
            break;
        }
    }

    assert!(
        done,
        "mock env should stop once max_episode_frames is reached"
    );
    assert_eq!(env.recorded_movie().total_frames(), 60);
}
