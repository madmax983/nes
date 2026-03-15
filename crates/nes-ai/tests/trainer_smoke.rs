use std::{fs, path::PathBuf};

use burn_core::{
    module::Module,
    record::{DefaultRecorder, FileRecorder},
};
use burn_ndarray::NdArray;
use burn_tensor::backend::Backend;
use nes_core::NesCore;
use tempfile::tempdir;

use nes_ai::{
    actions::ControlAction,
    config::{AiProfileConfig, GameProfileId, ObservationConfig, RewardConfig},
    env::ObservationSnapshot,
    error::AiError,
    model::HybridPolicyValueConfig,
    snapshot::{sha256_hex, write_snapshot_bundle},
    trainer::{
        TrainerConfig, action_from_arg, evaluate_control_profile, evaluate_random_policy,
        run_mock_ppo_smoke,
    },
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
    let mut checkpoint_file = trained.checkpoint_paths[0].clone();
    checkpoint_file.set_extension("mpk");

    assert_eq!(trained.checkpoint_paths.len(), 1);
    assert_eq!(trained.artifact_paths.len(), 2);
    assert!(checkpoint_file.exists());
    assert!(trained.artifact_paths[0].tas_json_path.exists());
    assert!(trained.artifact_paths[0].run_json_path.exists());
}

#[test]
fn ppo_smoke_returns_checkpoint_base_paths() {
    let dir = tempdir().unwrap();
    let cfg = TrainerConfig {
        training_updates: 1,
        checkpoint_interval: 1,
        checkpoint_dir: Some(dir.path().join("checkpoints")),
        ..TrainerConfig::smoke()
    };

    let trained = run_mock_ppo_smoke(&cfg, 1).unwrap();
    let checkpoint_base = &trained.checkpoint_paths[0];
    let mut checkpoint_file = checkpoint_base.clone();
    checkpoint_file.set_extension("mpk");

    assert!(
        checkpoint_base.extension().is_none(),
        "training should return checkpoint base paths that evaluate_smb_control can reload"
    );
    assert!(checkpoint_file.exists());
}

#[test]
fn ppo_smoke_returned_checkpoint_base_round_trips_through_burn_load_file() {
    let dir = tempdir().unwrap();
    let cfg = TrainerConfig {
        training_updates: 1,
        checkpoint_interval: 1,
        checkpoint_dir: Some(dir.path().join("checkpoints")),
        ..TrainerConfig::smoke()
    };

    let trained = run_mock_ppo_smoke(&cfg, 1).unwrap();
    let checkpoint_base = trained.checkpoint_paths[0].clone();
    let checkpoint_file = checkpoint_base
        .with_extension(<DefaultRecorder as FileRecorder<NdArray<f32>>>::file_extension());
    let observation = ObservationSnapshot {
        frame_stack: 4,
        width: 20,
        height: 20,
        frames: vec![0.0; 4 * 20 * 20],
        features: vec![0.0; 6],
    };
    let model_cfg =
        HybridPolicyValueConfig::from_observation(&observation, ControlAction::action_count());
    let device = <NdArray<f32> as Backend>::Device::default();
    let recorder = DefaultRecorder::new();

    assert!(checkpoint_file.exists());
    assert!(
        model_cfg
            .init::<NdArray<f32>>(&device)
            .load_file(checkpoint_base, &recorder, &device)
            .is_ok(),
        "returned checkpoint path should be reusable as evaluate_smb_control input"
    );
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
fn generic_profile_evaluator_dispatches_smb_configs() {
    let dir = tempdir().unwrap();
    let rom_path = dir.path().join("mock.nes");
    let snapshot_path = dir.path().join("mock.state.json");
    let rom_bytes = b"mock-rom";
    fs::write(&rom_path, rom_bytes).unwrap();

    let mut core = NesCore::new();
    core.load_cpu_bytes(0xC000, &[0xEA, 0x4C, 0x00, 0xC0]);
    write_snapshot_bundle(
        &snapshot_path,
        &sha256_hex(rom_bytes),
        "mock-v1",
        &core.save_state(),
    )
    .unwrap();

    let cfg = AiProfileConfig {
        game: GameProfileId::Smb,
        id: "mock-control".to_owned(),
        rom_path,
        snapshot_path,
        bootstrap_tas_path: PathBuf::from("mock.tas.json"),
        frame_stack: 4,
        frame_skip: 1,
        max_episode_frames: 30,
        observation: ObservationConfig {
            width: 84,
            height: 84,
        },
        reward: RewardConfig {
            forward_progress: 1.0,
            alive_bonus: 0.01,
            stall_penalty: -0.02,
            death_penalty: -1.0,
            stall_frames: 30,
        },
    };

    let summary = evaluate_control_profile(&cfg, &TrainerConfig::smoke(), 1, None).unwrap();

    assert!(summary.average_return.is_finite());
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
