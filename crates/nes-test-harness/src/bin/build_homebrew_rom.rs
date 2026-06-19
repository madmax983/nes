//! Homebrew ROM builder binary
//!
//! A standalone utility used to assemble and write the custom homebrew NES ROM.
//! This allows the workspace to have a deterministic test target without needing
//! an external 6502 assembler.

use std::env;
use std::path::PathBuf;

use comfy_table::{Cell, Color as TableColor, Table, presets::UTF8_FULL};
use crossterm::style::{Color, Stylize};
use nes_test_harness::{default_homebrew_rom_path, write_homebrew_rom};

fn main() {
    if let Err(err) = run() {
        eprintln!("\n{}", err);
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    run_with_args(env::args().skip(1))
}

fn run_with_args(mut args: impl Iterator<Item = String>) -> Result<(), String> {
    let mut out_path = default_homebrew_rom_path();

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--help" | "-h" => {
                println!(
                    "Usage: build_homebrew_rom [--out <path>]\nDefault output: {}",
                    out_path.display()
                );
                return Ok(());
            }
            "--out" => {
                let Some(path) = args.next() else {
                    return Err("missing value after --out".to_owned());
                };
                out_path = PathBuf::from(path);
            }
            _ => return Err(format!("unknown argument '{arg}'")),
        }
    }

    write_homebrew_rom(&out_path)?;

    println!("{}", "Building Homebrew ROM...".with(Color::Cyan).bold());

    println!("\n{}", build_success_table(&out_path));

    Ok(())
}

fn build_success_table(out_path: &std::path::Path) -> Table {
    let mut table = Table::new();
    table.load_preset(UTF8_FULL);
    table.set_header(vec![
        Cell::new("Property").fg(TableColor::Cyan),
        Cell::new("Value").fg(TableColor::White),
    ]);

    table.add_row(vec![
        Cell::new("Output Path"),
        Cell::new(out_path.display().to_string()).fg(TableColor::Green),
    ]);
    table.add_row(vec![
        Cell::new("Status"),
        Cell::new("Success").fg(TableColor::Green),
    ]);

    table
}

#[cfg(test)]
mod tests {
    use super::{build_success_table, run_with_args};
    use std::path::PathBuf;

    #[test]
    fn build_success_table_includes_path_and_success_status() {
        let path = PathBuf::from("test/path/rom.nes");
        let table = build_success_table(&path);
        let output = table.to_string();

        assert!(output.contains("test/path/rom.nes"));
        assert!(output.contains("Output Path"));
        assert!(output.contains("Status"));
        assert!(output.contains("Success"));
    }

    #[test]
    fn run_with_args_help_returns_ok() {
        assert!(run_with_args(vec!["--help".to_string()].into_iter()).is_ok());
        assert!(run_with_args(vec!["-h".to_string()].into_iter()).is_ok());
    }

    #[test]
    fn run_with_args_builds_homebrew_rom() {
        let temp_dir = std::env::temp_dir().join("nes_test_harness_build_homebrew_test");
        let _ = std::fs::remove_dir_all(&temp_dir);
        std::fs::create_dir_all(&temp_dir).unwrap();

        let out_path = temp_dir.join("test_homebrew.nes");

        let args = vec!["--out".to_string(), out_path.to_str().unwrap().to_string()];

        let result = run_with_args(args.into_iter());
        assert!(result.is_ok(), "Run should succeed");

        assert!(out_path.exists(), "ROM file should have been written");

        let _ = std::fs::remove_dir_all(&temp_dir);
    }
}
