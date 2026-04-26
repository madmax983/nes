//! Evaluation binary for SMB control policies
//!
//! A utility to evaluate a trained model checkpoint against the environment.
//! It executes the policy and writes deterministic TAS replay artifacts.

use std::{env, fs, path::PathBuf};

use comfy_table::{Cell, Color as TableColor, Table, presets::UTF8_FULL};
use crossterm::style::{Color, Stylize};
use nes_ai::{
    config::AiProfileConfig,
    trainer::{TrainerConfig, evaluate_smb_control},
};

fn format_config_read_error(path: &std::path::Path, err: &std::io::Error) -> String {
    if err.kind() == std::io::ErrorKind::NotFound {
        format!(
            "{} Could not find the profile config at '{}'.\n{} Ensure the file exists and the path is correct.",
            "Error:".with(Color::Red).bold(),
            path.display().to_string().with(Color::Yellow),
            "Hint:".with(Color::Cyan).bold()
        )
    } else {
        format!(
            "{} Failed to read profile config at '{}': {}",
            "Error:".with(Color::Red).bold(),
            path.display(),
            err
        )
    }
}

fn main() {
    if let Err(err) = run() {
        eprintln!("\n{err}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let args = env::args().collect::<Vec<_>>();
    if args.iter().any(|arg| arg == "--help" || arg == "-h") {
        println!(
            "Usage: eval_smb_control <profile_toml> <checkpoint_base> [episodes] [artifact_dir]"
        );
        std::process::exit(0);
    }
    if args.len() < 3 || args.len() > 5 {
        return Err(format!(
            "{} missing or invalid number of arguments.\nUsage: eval_smb_control <profile_toml> <checkpoint_base> [episodes] [artifact_dir]",
            "Error:".with(Color::Red).bold()
        ));
    }

    let profile_path = PathBuf::from(&args[1]);
    let checkpoint_base = PathBuf::from(&args[2]);
    let episodes = args
        .get(3)
        .map(|value| value.parse::<usize>())
        .transpose()
        .map_err(|e| format!("Failed to parse episodes: {e}"))?
        .unwrap_or(2);
    let artifact_dir = args.get(4).map(PathBuf::from);

    let profile_str = fs::read_to_string(&profile_path)
        .map_err(|e| format_config_read_error(&profile_path, &e))?;
    let profile_cfg: AiProfileConfig = toml::from_str(&profile_str).map_err(|e| {
        format!(
            "{} Failed to parse profile config:\n{}",
            "Error:".with(Color::Red).bold(),
            e
        )
    })?;

    let trainer_cfg = TrainerConfig {
        artifact_dir: artifact_dir.clone(),
        ..TrainerConfig::smoke()
    };

    println!("{}", "Evaluating AI Profile...".with(Color::Cyan).bold());

    let summary =
        evaluate_smb_control(&profile_cfg, &trainer_cfg, episodes, Some(&checkpoint_base))
            .map_err(|e| {
                format!(
                    "{} Evaluation failed:\n  {}",
                    "Error:".with(Color::Red).bold(),
                    e
                )
            })?;

    println!(
        "\n{}",
        build_summary_table(summary.average_return, summary.artifact_paths.len())
    );

    Ok(())
}

fn build_summary_table(average_return: f32, artifact_paths_len: usize) -> Table {
    let mut table = Table::new();
    table.load_preset(UTF8_FULL);
    table.set_header(vec![
        Cell::new("Property").fg(TableColor::Cyan),
        Cell::new("Value").fg(TableColor::White),
    ]);

    table.add_row(vec![
        Cell::new("Average Return"),
        Cell::new(format!("{average_return:.2}")).fg(TableColor::Yellow),
    ]);
    table.add_row(vec![
        Cell::new("Artifacts Written"),
        Cell::new(artifact_paths_len.to_string()).fg(TableColor::Yellow),
    ]);

    table
}

#[cfg(test)]
mod tests {
    use super::build_summary_table;

    #[test]
    fn build_summary_table_formats_average_return_and_artifacts() {
        let table = build_summary_table(2.5, 3);
        let output = table.to_string();

        assert!(output.contains("Average Return"));
        assert!(output.contains("2.50"));
        assert!(output.contains("Artifacts Written"));
        assert!(output.contains('3'));
    }
}
