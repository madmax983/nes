use std::{env, fs, path::PathBuf};

use comfy_table::{Cell, Color as TableColor, Table};
use crossterm::style::{Color, Stylize};
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
    let profile_cfg: AiProfileConfig = toml::from_str(&fs::read_to_string(&profile_path)?)?;
    let trainer_cfg = TrainerConfig {
        artifact_dir: artifact_dir.clone(),
        ..TrainerConfig::smoke()
    };

    println!("{}", "Evaluating AI Profile...".with(Color::Cyan).bold());

    let summary =
        evaluate_smb_control(&profile_cfg, &trainer_cfg, episodes, Some(&checkpoint_base))?;

    let mut table = Table::new();
    table.set_header(vec![
        Cell::new("Metric").fg(TableColor::Cyan),
        Cell::new("Value").fg(TableColor::White),
    ]);

    table.add_row(vec![
        Cell::new("Average Return"),
        Cell::new(format!("{:.2}", summary.average_return)).fg(TableColor::Yellow),
    ]);
    table.add_row(vec![
        Cell::new("Artifacts Written"),
        Cell::new(summary.artifact_paths.len().to_string()).fg(TableColor::Green),
    ]);

    println!("\n{table}");

    Ok(())
}
