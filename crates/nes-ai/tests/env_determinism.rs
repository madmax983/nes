mod support;

use std::{fs, path::PathBuf};

use nes_ai::{
    actions::ControlAction,
    config::{AiProfileConfig, GameProfileId, ObservationConfig, RewardConfig},
    env::{AnyControlEnv, SmbControlEnv},
    error::AiError,
    snapshot::write_snapshot_bundle,
};
use nes_core::NesCore;
use sha2::{Digest, Sha256};
use support::mock_profile::make_mock_env;
use tempfile::tempdir;

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

#[test]
fn from_config_rejects_observation_dims_below_model_minimum() {
    let cfg = AiProfileConfig {
        game: GameProfileId::Smb,
        id: "mock-control".to_owned(),
        rom_path: PathBuf::from("mock.nes"),
        snapshot_path: PathBuf::from("mock.state.json"),
        bootstrap_tas_path: PathBuf::from("mock.tas.json"),
        frame_stack: 4,
        frame_skip: 1,
        max_episode_frames: 60,
        observation: ObservationConfig {
            width: 19,
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

    let Err(err) = SmbControlEnv::from_config(cfg) else {
        panic!("observation dims below model minimum should fail fast");
    };

    assert!(matches!(
        err,
        AiError::Unsupported("observation width must be at least 20 pixels")
    ));
}

#[test]
fn from_config_rejects_rom_hash_mismatch() {
    let dir = tempdir().unwrap();
    let rom_path = dir.path().join("mock.nes");
    let snapshot_path = dir.path().join("mock.state.json");
    fs::write(&rom_path, b"live-rom").unwrap();

    let snapshot = NesCore::new().save_state();
    let expected_hash = sha256_hex(b"snapshot-rom");
    let found_hash = sha256_hex(b"live-rom");
    write_snapshot_bundle(&snapshot_path, &expected_hash, "mock-v1", &snapshot).unwrap();

    let cfg = AiProfileConfig {
        game: GameProfileId::Smb,
        id: "mock-control".to_owned(),
        rom_path,
        snapshot_path,
        bootstrap_tas_path: PathBuf::from("mock.tas.json"),
        frame_stack: 4,
        frame_skip: 1,
        max_episode_frames: 60,
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

    let Err(err) = SmbControlEnv::from_config(cfg) else {
        panic!("mismatched ROM should fail fast");
    };

    assert!(matches!(
        err,
        AiError::RomHashMismatch { expected, found }
            if expected == expected_hash && found == found_hash
    ));
}

#[test]
fn any_control_env_dispatches_smb_configs() {
    let dir = tempdir().unwrap();
    let rom_path = dir.path().join("mock.nes");
    let snapshot_path = dir.path().join("mock.state.json");
    let rom_bytes = b"mock-rom";
    fs::write(&rom_path, rom_bytes).unwrap();

    let mut core = NesCore::new();
    core.load_cpu_bytes(0xC000, &[0xEA, 0x4C, 0x00, 0xC0]);
    let snapshot = core.save_state();
    write_snapshot_bundle(&snapshot_path, &sha256_hex(rom_bytes), "mock-v1", &snapshot).unwrap();

    let cfg = AiProfileConfig {
        game: GameProfileId::Smb,
        id: "mock-control".to_owned(),
        rom_path,
        snapshot_path,
        bootstrap_tas_path: PathBuf::from("mock.tas.json"),
        frame_stack: 4,
        frame_skip: 1,
        max_episode_frames: 60,
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

fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    format!("{digest:x}")
}
