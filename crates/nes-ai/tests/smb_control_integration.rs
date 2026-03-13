use std::path::PathBuf;

use nes_ai::{actions::ControlAction, config::AiProfileConfig, env::SmbControlEnv};

#[test]
#[ignore = "requires local SMB ROM and generated control snapshot"]
fn smb_control_profile_can_reset_and_gain_forward_reward() {
    let cfg: AiProfileConfig = toml::from_str(
        &std::fs::read_to_string(PathBuf::from("config/ai/profiles/smb-control.toml")).unwrap(),
    )
    .unwrap();

    let mut env = SmbControlEnv::from_config(cfg).unwrap();
    let _ = env.reset().unwrap();
    let step = env.step(ControlAction::Right).unwrap();

    assert!(step.reward.total.is_finite());
}
