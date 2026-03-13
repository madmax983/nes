use std::path::PathBuf;

use tempfile::tempdir;

use nes_ai::{
    actions::ControlAction,
    config::AiProfileConfig,
    env::SmbControlEnv,
    trainer::{TrainerConfig, train_smb_control},
};

#[test]
#[ignore = "requires local SMB ROM and generated control snapshot"]
fn smb_control_profile_can_reset_and_gain_forward_reward() {
    let cfg: AiProfileConfig = toml::from_str(
        &std::fs::read_to_string(PathBuf::from("config/ai/profiles/smb-control.toml")).unwrap(),
    )
    .unwrap();

    let mut env = SmbControlEnv::from_config(cfg).unwrap();
    let _ = env.reset().unwrap();
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
    let cfg: AiProfileConfig = toml::from_str(
        &std::fs::read_to_string(PathBuf::from("config/ai/profiles/smb-control.toml")).unwrap(),
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

    let trained = train_smb_control(&cfg, &trainer_cfg, 1).unwrap();

    assert_eq!(trained.checkpoint_paths.len(), 1);
    assert_eq!(trained.artifact_paths.len(), 1);
    assert!(trained.checkpoint_paths[0].exists());
    assert!(trained.artifact_paths[0].tas_json_path.exists());
    assert!(trained.artifact_paths[0].run_json_path.exists());
}
