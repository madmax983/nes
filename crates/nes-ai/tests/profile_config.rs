use nes_ai::config::{AiProfileConfig, GameProfileId};

#[test]
fn profile_config_parses_expected_control_defaults() {
    let cfg: AiProfileConfig = toml::from_str(sample_profile_toml()).unwrap();

    assert_eq!(cfg.game, GameProfileId::Smb);
    assert_eq!(cfg.frame_stack, 4);
    assert_eq!(cfg.observation.width, 84);
    assert_eq!(cfg.reward.stall_frames, 120);
}

#[test]
fn profile_config_parses_explicit_game_id() {
    let profile = format!("game = \"smb\"\n{}", sample_profile_toml());

    let cfg: AiProfileConfig = toml::from_str(&profile).unwrap();

    assert_eq!(cfg.game, GameProfileId::Smb);
}

#[test]
fn profile_config_rejects_unknown_game_id() {
    let profile = format!("game = \"metroid\"\n{}", sample_profile_toml());
    let err = toml::from_str::<AiProfileConfig>(&profile).unwrap_err();

    assert!(err.to_string().contains("game"));
}

#[test]
fn profile_config_rejects_zero_frame_stack() {
    let profile = sample_profile_toml().replace("frame_stack = 4", "frame_stack = 0");
    let err = toml::from_str::<AiProfileConfig>(&profile).unwrap_err();

    assert!(err.to_string().contains("greater than zero"));
}

#[test]
fn profile_config_rejects_zero_observation_width() {
    let profile = sample_profile_toml().replace("width = 84", "width = 0");
    let err = toml::from_str::<AiProfileConfig>(&profile).unwrap_err();

    assert!(err.to_string().contains("greater than zero"));
}

#[test]
fn profile_config_rejects_too_small_observation_width() {
    let profile = sample_profile_toml().replace("width = 84", "width = 19");

    let err = toml::from_str::<AiProfileConfig>(&profile).expect_err("width below 20 should fail");

    assert!(err.to_string().contains("at least 20"));
}

#[test]
fn profile_config_rejects_too_small_observation_height() {
    let profile = sample_profile_toml().replace("height = 84", "height = 19");

    let err = toml::from_str::<AiProfileConfig>(&profile).expect_err("height below 20 should fail");

    assert!(err.to_string().contains("at least 20"));
}

fn sample_profile_toml() -> &'static str {
    r#"
id = "smb-control"
rom_path = "roms/smb.nes"
snapshot_path = "artifacts/smb.state.json"
bootstrap_tas_path = "crates/nes-ai/assets/bootstrap/smb_1_1_entry.tas.json"
frame_stack = 4
frame_skip = 4
max_episode_frames = 900

[observation]
width = 84
height = 84

[reward]
forward_progress = 1.0
alive_bonus = 0.01
stall_penalty = -0.02
death_penalty = -1.0
stall_frames = 120
"#
}
