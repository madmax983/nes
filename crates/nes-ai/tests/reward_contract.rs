use nes_ai::reward::{RewardBreakdown, RewardModel};
use nes_ai::{config::RewardConfig, profiles::smb::SmbFeatures};

#[test]
fn forward_progress_beats_stall_and_death_is_terminal() {
    let reward = RewardConfig {
        forward_progress: 1.0,
        alive_bonus: 0.01,
        stall_penalty: -0.02,
        death_penalty: -1.0,
        stall_frames: 120,
    };

    let model = RewardModel::new(reward);
    let prev = SmbFeatures {
        level_progress: 10.0,
        horizontal_speed: 0.0,
        vertical_speed: 0.0,
        airborne: false,
        player_state: 0x08,
        lives: 3,
    };
    let next = SmbFeatures {
        level_progress: 20.0,
        ..prev.clone()
    };

    let RewardBreakdown { total, done, .. } = model.score(&prev, &next, 0);
    assert!(total > 0.0);
    assert!(!done);

    let dead = SmbFeatures {
        player_state: 0x0B,
        ..next
    };
    let RewardBreakdown { total, done, .. } = model.score(&prev, &dead, 0);
    assert!(done);
    assert!(total <= reward.death_penalty);
}
