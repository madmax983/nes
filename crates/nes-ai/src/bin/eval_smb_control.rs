use std::{env, fs, path::PathBuf};

use nes_ai::{
    config::AiProfileConfig,
    trainer::{TrainerConfig, evaluate_smb_control},
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = env::args().collect::<Vec<_>>();
    if args.len() < 3 || args.len() > 5 {
        eprintln!(
            "Usage: eval_smb_control <profile_toml> <checkpoint_base> [episodes] [artifact_dir]"
        );
        std::process::exit(2);
    }

    let profile_path = PathBuf::from(&args[1]);
    let checkpoint_base = PathBuf::from(&args[2]);
    let episodes = args
        .get(3)
        .map(|value| value.parse::<usize>())
        .transpose()?
        .unwrap_or(2);
    let artifact_dir = args.get(4).map(PathBuf::from);
    let profile_cfg: AiProfileConfig = toml::from_str(&fs::read_to_string(profile_path)?)?;
    let trainer_cfg = TrainerConfig {
        artifact_dir,
        ..TrainerConfig::smoke()
    };

    let summary =
        evaluate_smb_control(&profile_cfg, &trainer_cfg, episodes, Some(&checkpoint_base))?;
    println!("average_return={}", summary.average_return);
    println!("artifacts_written={}", summary.artifact_paths.len());
    Ok(())
}
