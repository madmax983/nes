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
    let mut args = env::args().skip(1);
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
                    return Err(format!(
                        "{} missing value after --out",
                        "Error:".with(Color::Red).bold()
                    ));
                };
                out_path = PathBuf::from(path);
            }
            _ => {
                return Err(format!(
                    "{} unknown argument '{}'",
                    "Error:".with(Color::Red).bold(),
                    arg
                ));
            }
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
    use super::build_success_table;
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
}
