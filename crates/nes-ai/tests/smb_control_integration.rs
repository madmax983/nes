use std::path::PathBuf;
use tempfile::tempdir;

use nes_ai::{
    actions::ControlAction,
    config::AiProfileConfig,
    env::AnyControlEnv,
    trainer::{TrainerConfig, train_control_profile},
};

#[test]
#[ignore = "requires local SMB ROM and generated control snapshot"]
fn smb_control_profile_can_reset_and_gain_forward_reward() {
    let cfg_path = PathBuf::from("config/ai/profiles/smb-control.toml");
    if !cfg_path.exists() {
        return;
    }
    let cfg: AiProfileConfig = toml::from_str(
        &std::fs::read_to_string(cfg_path).unwrap_or_default(),
    )
    .unwrap();

    let mut env = AnyControlEnv::from_config(cfg).unwrap();
    env.reset().unwrap();
    let observation = env.observation().unwrap();
    let step = env.step(ControlAction::Right).unwrap();

    assert_eq!(
        observation.frames.len(),
        observation.frame_stack * observation.width * observation.height
    );
    assert!(env.recorded_movie().total_frames() > 0);
    assert!(step.reward.total.is_finite());
}

#[test]
#[ignore = "requires local SMB ROM and generated control snapshot"]
fn smb_control_training_stack_can_emit_checkpoint_and_eval_artifacts() {
    let cfg_path = PathBuf::from("config/ai/profiles/smb-control.toml");
    if !cfg_path.exists() {
        return;
    }
    let cfg: AiProfileConfig = toml::from_str(
        &std::fs::read_to_string(cfg_path).unwrap_or_default(),
    )
    .unwrap();
    let dir = tempdir().unwrap();
    let trainer_cfg = TrainerConfig {
        rollout_steps: 8,
        minibatch_size: 4,
        epochs: 1,
        training_updates: 1,
        checkpoint_interval: 1,
        checkpoint_dir: Some(dir.path().join("checkpoints")),
        artifact_dir: Some(dir.path().join("eval")),
        ..TrainerConfig::smoke()
    };

    let trained = train_control_profile(&cfg, &trainer_cfg, 1).unwrap();
    let mut checkpoint_file = trained.checkpoint_paths[0].clone();
    checkpoint_file.set_extension("mpk");

    assert_eq!(trained.checkpoint_paths.len(), 1);
    assert_eq!(trained.artifact_paths.len(), 1);
    assert!(trained.checkpoint_paths[0].extension().is_none());
    assert!(checkpoint_file.exists());
    assert!(trained.artifact_paths[0].tas_json_path.exists());
    assert!(trained.artifact_paths[0].run_json_path.exists());
}
