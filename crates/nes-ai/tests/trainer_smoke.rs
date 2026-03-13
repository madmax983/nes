use tempfile::tempdir;

use nes_ai::{
    actions::ControlAction,
    error::AiError,
    trainer::{TrainerConfig, action_from_arg, evaluate_random_policy, run_mock_ppo_smoke},
};

#[test]
fn ppo_smoke_produces_positive_finite_return() {
    let cfg = TrainerConfig::smoke();
    let trained = run_mock_ppo_smoke(&cfg, 4).unwrap();

    assert!(trained.average_return.is_finite());
    assert!(trained.average_return > 0.0);
}

#[test]
fn ppo_smoke_return_changes_with_training_budget() {
    let cfg = TrainerConfig::smoke();
    let stronger = TrainerConfig {
        training_updates: cfg.training_updates + 1,
        epochs: cfg.epochs + 1,
        ..cfg.clone()
    };

    let baseline = run_mock_ppo_smoke(&cfg, 4).unwrap();
    let stronger = run_mock_ppo_smoke(&stronger, 4).unwrap();

    assert!(stronger.average_return > baseline.average_return);
}

#[test]
fn ppo_smoke_writes_checkpoints_and_eval_artifacts() {
    let dir = tempdir().unwrap();
    let cfg = TrainerConfig {
        training_updates: 1,
        checkpoint_interval: 1,
        checkpoint_dir: Some(dir.path().join("checkpoints")),
        artifact_dir: Some(dir.path().join("eval")),
        ..TrainerConfig::smoke()
    };

    let trained = run_mock_ppo_smoke(&cfg, 2).unwrap();

    assert_eq!(trained.checkpoint_paths.len(), 1);
    assert_eq!(trained.artifact_paths.len(), 2);
    assert!(trained.checkpoint_paths[0].exists());
    assert!(trained.artifact_paths[0].tas_json_path.exists());
    assert!(trained.artifact_paths[0].run_json_path.exists());
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
