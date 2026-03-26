use std::{env, fs, path::PathBuf};

use comfy_table::{Cell, Color as TableColor, Table};
use crossterm::style::{Color, Stylize};
use nes_ai::{
    config::AiProfileConfig,
    trainer::{TrainerConfig, train_smb_control},
};

fn main() {
    if let Err(err) = run() {
        eprintln!("\n{}", format!("Error: {err}").with(Color::Red).bold());
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let args = env::args().collect::<Vec<_>>();
    if args.iter().any(|arg| arg == "--help" || arg == "-h") {
        println!(
            "Usage: train_smb_control <profile_toml> [episodes] [checkpoint_dir] [artifact_dir]"
        );
        std::process::exit(0);
    }
    if args.len() < 2 || args.len() > 5 {
        println!("Usage: train_smb_control <profile_toml> [episodes] [checkpoint_dir] [artifact_dir]");
        std::process::exit(1);
    }

    let profile_path = PathBuf::from(&args[1]);
    let episodes = args
        .get(2)
        .map(|value| value.parse::<usize>())
        .transpose()
        .map_err(|e| format!("Failed to parse episodes: {e}"))?
        .unwrap_or(4);
    let checkpoint_dir = args.get(3).map(PathBuf::from);
    let artifact_dir = args.get(4).map(PathBuf::from);

    let profile_str = fs::read_to_string(&profile_path)
        .map_err(|e| format!("Failed to read profile config: {e}"))?;
    let profile_cfg: AiProfileConfig =
        toml::from_str(&profile_str).map_err(|e| format!("Failed to parse profile config: {e}"))?;

    let trainer_cfg = TrainerConfig {
        checkpoint_dir: checkpoint_dir.clone(),
        artifact_dir: artifact_dir.clone(),
        ..TrainerConfig::smoke()
    };

    println!("{}", "Training AI Profile...".with(Color::Cyan).bold());

    let summary = train_smb_control(&profile_cfg, &trainer_cfg, episodes)
        .map_err(|e| format!("Training failed: {e}"))?;

    println!(
        "\n{}",
        build_summary_table(
            summary.average_return,
            summary.checkpoint_paths.len(),
            summary.artifact_paths.len()
        )
    );

    Ok(())
}

fn build_summary_table(
    average_return: f32,
    checkpoint_paths_len: usize,
    artifact_paths_len: usize,
) -> Table {
    let mut table = Table::new();
    table.set_header(vec![
        Cell::new("Metric").fg(TableColor::Cyan),
        Cell::new("Value").fg(TableColor::White),
    ]);

    table.add_row(vec![
        Cell::new("Average Return"),
        Cell::new(format!("{average_return:.2}")).fg(TableColor::Yellow),
    ]);
    table.add_row(vec![
        Cell::new("Checkpoints Written"),
        Cell::new(checkpoint_paths_len.to_string()).fg(TableColor::Green),
    ]);
    table.add_row(vec![
        Cell::new("Artifacts Written"),
        Cell::new(artifact_paths_len.to_string()).fg(TableColor::Green),
    ]);

    table
}

#[cfg(test)]
mod tests {
    use super::build_summary_table;

    #[test]
    fn build_summary_table_formats_metrics() {
        let table = build_summary_table(2.5, 3, 4);
        let output = table.to_string();

        assert!(output.contains("Average Return"));
        assert!(output.contains("2.50"));
        assert!(output.contains("Checkpoints Written"));
        assert!(output.contains('3'));
        assert!(output.contains("Artifacts Written"));
        assert!(output.contains('4'));
    }
}
