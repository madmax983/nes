use nes_ai::{
    actions::ControlAction,
    error::AiError,
    trainer::{TrainerConfig, action_from_arg, evaluate_random_policy, run_mock_ppo_smoke},
};

#[test]
fn ppo_smoke_improves_mock_env_return_over_random_baseline() {
    let cfg = TrainerConfig::smoke();
    let baseline = evaluate_random_policy(&cfg, 8).unwrap();
    let trained = run_mock_ppo_smoke(&cfg, 8).unwrap();

    assert!(trained.average_return >= baseline.average_return);
}

#[test]
fn ppo_smoke_return_changes_with_training_budget() {
    let cfg = TrainerConfig::smoke();
    let stronger = TrainerConfig {
        epochs: cfg.epochs + 2,
        ..cfg.clone()
    };

    let baseline = run_mock_ppo_smoke(&cfg, 8).unwrap();
    let stronger = run_mock_ppo_smoke(&stronger, 8).unwrap();

    assert!(stronger.average_return > baseline.average_return);
}

#[test]
fn trainer_rejects_zero_episode_requests() {
    let cfg = TrainerConfig::smoke();

    let err = evaluate_random_policy(&cfg, 0).unwrap_err();

    assert!(matches!(
        err,
        AiError::Unsupported("episodes must be greater than zero")
    ));
}

#[test]
fn action_parser_maps_known_labels() {
    assert_eq!(action_from_arg("noop").unwrap(), ControlAction::Noop);
    assert_eq!(action_from_arg("right").unwrap(), ControlAction::Right);
    assert_eq!(action_from_arg("right-a").unwrap(), ControlAction::RightA);
    assert_eq!(action_from_arg("a").unwrap(), ControlAction::A);
    assert_eq!(action_from_arg("right-b").unwrap(), ControlAction::RightB);
    assert_eq!(
        action_from_arg("right-a-b").unwrap(),
        ControlAction::RightAB
    );
}

#[test]
fn action_parser_rejects_unknown_labels() {
    let err = action_from_arg("left").unwrap_err();

    assert!(matches!(err, AiError::Unsupported("unknown action")));
}
