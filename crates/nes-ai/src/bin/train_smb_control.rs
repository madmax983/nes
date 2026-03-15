use std::{env, fs, path::PathBuf};

use nes_ai::{
    config::AiProfileConfig,
    trainer::{TrainerConfig, train_smb_control},
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = env::args().collect::<Vec<_>>();
    if args.len() < 2 || args.len() > 5 {
        eprintln!(
            "Usage: train_smb_control <profile_toml> [episodes] [checkpoint_dir] [artifact_dir]"
        );
        std::process::exit(2);
    }

    let profile_path = PathBuf::from(&args[1]);
    let episodes = args
        .get(2)
        .map(|value| value.parse::<usize>())
        .transpose()?
        .unwrap_or(4);
    let checkpoint_dir = args.get(3).map(PathBuf::from);
    let artifact_dir = args.get(4).map(PathBuf::from);
    let profile_cfg: AiProfileConfig = toml::from_str(&fs::read_to_string(profile_path)?)?;
    let trainer_cfg = TrainerConfig {
        checkpoint_dir,
        artifact_dir,
        ..TrainerConfig::smoke()
    };

    let summary = train_smb_control(&profile_cfg, &trainer_cfg, episodes)?;
    println!("average_return={}", summary.average_return);
    println!("checkpoints_written={}", summary.checkpoint_paths.len());
    println!("artifacts_written={}", summary.artifact_paths.len());
    Ok(())
}
