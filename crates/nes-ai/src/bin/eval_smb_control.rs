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

fn main() {
    if let Err(err) = run() {
        eprintln!("\n{err}");
        std::process::exit(1);
    }
}

fn format_profile_read_error(profile_path: &str, err: &std::io::Error) -> String {
    if err.kind() == std::io::ErrorKind::NotFound {
        format!(
            "{} Could not find the profile config file at '{}'.\n{} Copy the example profile: cp config/ai/profiles/smb-control.example.toml {}",
            "Error:".with(Color::Red).bold(),
            profile_path.with(Color::Yellow),
            "Hint:".with(Color::Cyan).bold(),
            profile_path.with(Color::Yellow)
        )
    } else {
        format!(
            "{} Failed to read profile config at '{}': {}",
            "Error:".with(Color::Red).bold(),
            profile_path,
            err
        )
    }
}

fn format_eval_error(err: String) -> String {
    if err.contains("No such file or directory") && err.contains(".state.json") {
        format!(
            "{} {}\n{} You may need to generate the snapshot first. Run:\n  cargo run -p nes-ai --bin prepare_smb_control -- ./roms/homebrew/homebrew.nes ./crates/nes-ai/assets/bootstrap/smb_1_1_entry.tas.json ./artifacts/ai/snapshots/smb-1-1-control.state.json",
            "Error:".with(Color::Red).bold(),
            err.with(Color::Red),
            "Hint:".with(Color::Cyan).bold()
        )
    } else {
        format!("{} {}", "Error:".with(Color::Red).bold(), err)
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
        eprintln!(
            "Usage: eval_smb_control <profile_toml> <checkpoint_base> [episodes] [artifact_dir]"
        );
        std::process::exit(1);
    }

    let profile_path = PathBuf::from(&args[1]);
    let checkpoint_base = PathBuf::from(&args[2]);
    let episodes = args
        .get(3)
        .map(|value| value.parse::<usize>())
        .transpose()
        .map_err(|e| format!("{} Failed to parse episodes: {}", "Error:".with(Color::Red).bold(), e))?
        .unwrap_or(2);
    let artifact_dir = args.get(4).map(PathBuf::from);

    let profile_str = fs::read_to_string(&profile_path)
        .map_err(|e| format_profile_read_error(&profile_path.to_string_lossy(), &e))?;
    let profile_cfg: AiProfileConfig =
        toml::from_str(&profile_str).map_err(|e| format!("{} Failed to parse profile config: {}", "Error:".with(Color::Red).bold(), e))?;

    let trainer_cfg = TrainerConfig {
        artifact_dir: artifact_dir.clone(),
        ..TrainerConfig::smoke()
    };

    println!("{}", "Evaluating AI Profile...".with(Color::Cyan).bold());

    let summary =
        evaluate_smb_control(&profile_cfg, &trainer_cfg, episodes, Some(&checkpoint_base))
            .map_err(|e| format_eval_error(format!("Evaluation failed: {e}")))?;

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
        Cell::new("Metric").fg(TableColor::Cyan),
        Cell::new("Value").fg(TableColor::White),
    ]);

    table.add_row(vec![
        Cell::new("Average Return"),
        Cell::new(format!("{average_return:.2}")).fg(TableColor::Yellow),
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
    fn build_summary_table_formats_average_return_and_artifacts() {
        let table = build_summary_table(2.5, 3);
        let output = table.to_string();

        assert!(output.contains("Average Return"));
        assert!(output.contains("2.50"));
        assert!(output.contains("Artifacts Written"));
        assert!(output.contains('3'));
    }
}
