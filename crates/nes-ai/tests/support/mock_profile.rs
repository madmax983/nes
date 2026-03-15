use std::path::PathBuf;

use nes_ai::{
    config::{AiProfileConfig, GameProfileId, ObservationConfig, RewardConfig},
    env::ProfileEnv,
    profiles::smb::SmbProfile,
    snapshot::{SNAPSHOT_BUNDLE_VERSION, SnapshotBundle},
};
use nes_core::NesCore;

pub fn make_mock_env() -> ProfileEnv<SmbProfile> {
    let mut core = NesCore::new();
    core.load_cpu_bytes(0xC000, &[0xEA, 0x4C, 0x00, 0xC0]);

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

    let snapshot = SnapshotBundle {
        version: SNAPSHOT_BUNDLE_VERSION,
        rom_hash: "mock".to_owned(),
        snapshot_id: "mock-v1".to_owned(),
        snapshot: core.save_state(),
    };

    ProfileEnv::new(SmbProfile::new(cfg), snapshot)
}
