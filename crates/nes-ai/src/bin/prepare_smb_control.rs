//! Snapshot preparation binary for SMB control task
//!
//! A utility to bootstrap the reinforcement learning environment by saving
//! a snapshot state (e.g., SMB 1-1) required to train the control policy.

use std::{env, fs, path::PathBuf};

use comfy_table::{Cell, Color as TableColor, Table, presets::UTF8_FULL};
use crossterm::style::{Color, Stylize};
use nes_ai::snapshot::{sha256_hex, write_snapshot_bundle};
use nes_core::{NesCore, tas::TasMovie};

fn main() {
    if let Err(err) = run() {
        eprintln!("\n{err}");
        std::process::exit(1);
    }
}

fn format_tas_read_error(tas_path: &str, err: &std::io::Error) -> String {
    if err.kind() == std::io::ErrorKind::NotFound {
        format!(
            "{} Could not find the TAS json file at '{}'.
{} Check the path.",
            "Error:".with(Color::Red).bold(),
            tas_path.with(Color::Yellow),
            "Hint:".with(Color::Cyan).bold()
        )
    } else {
        format!(
            "{} Failed to read TAS json at '{}': {}",
            "Error:".with(Color::Red).bold(),
            tas_path.with(Color::Yellow),
            err
        )
    }
}

fn format_rom_read_error(rom_path: &str, err: &std::io::Error) -> String {
    if err.kind() == std::io::ErrorKind::NotFound {
        format!(
            "{} Could not find the ROM file at '{}'.\n{} Check the path or try the bundled homebrew ROM: ./roms/homebrew/homebrew.nes or <path-to-your-rom>.nes",
            "Error:".with(Color::Red).bold(),
            rom_path.with(Color::Yellow),
            "Hint:".with(Color::Cyan).bold()
        )
    } else {
        format!(
            "{} Failed to read ROM at '{}': {}",
            "Error:".with(Color::Red).bold(),
            rom_path.with(Color::Yellow),
            err
        )
    }
}

fn run() -> Result<(), String> {
    let args = env::args().collect::<Vec<_>>();
    if args.iter().any(|arg| arg == "--help" || arg == "-h") {
        println!("Usage: prepare_smb_control <rom_path> <bootstrap_tas_json> <output_snapshot>");
        std::process::exit(0);
    }
    if args.len() != 4 {
        return Err(format!(
            "{} missing or invalid number of arguments.
Usage: prepare_smb_control <rom_path> <bootstrap_tas_json> <output_snapshot>",
            "Error:".with(Color::Red).bold()
        ));
    }

    let rom_path = PathBuf::from(&args[1]);
    let movie_path = PathBuf::from(&args[2]);
    let out_path = PathBuf::from(&args[3]);

    let rom =
        fs::read(&rom_path).map_err(|e| format_rom_read_error(&rom_path.to_string_lossy(), &e))?;
    let rom_hash = sha256_hex(&rom);

    let movie_json = fs::read(&movie_path)
        .map_err(|e| format_tas_read_error(&movie_path.to_string_lossy(), &e))?;
    let movie: TasMovie = serde_json::from_slice(&movie_json)
        .map_err(|e| format!("Failed to parse TAS json: {e}"))?;

    let mut core = NesCore::new();
    core.load_ines_rom(&rom)
        .map_err(|e| format!("Failed to load ROM: {e}"))?;
    movie
        .replay(&mut core)
        .map_err(|e| format!("Failed to replay TAS: {e}"))?;

    write_snapshot_bundle(&out_path, &rom_hash, "smb-control-v1", &core.save_state())
        .map_err(|e| format!("Failed to write snapshot bundle: {e}"))?;

    println!(
        "{}",
        "Prepared SMB Control Snapshot".with(Color::Cyan).bold()
    );

    println!("\n{}", build_success_table(&rom_hash, &out_path));

    Ok(())
}

fn build_success_table(rom_hash: &str, out_path: &std::path::Path) -> Table {
    let mut table = Table::new();
    table.load_preset(UTF8_FULL);
    table.set_header(vec![
        Cell::new("Property").fg(TableColor::Cyan),
        Cell::new("Value").fg(TableColor::White),
    ]);

    table.add_row(vec![
        Cell::new("ROM Hash"),
        Cell::new(rom_hash).fg(TableColor::Green),
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
    fn build_success_table_formats_hash_and_path() {
        let hash = "deadbeef";
        let path = PathBuf::from("test/path/snapshot.bin");
        let table = build_success_table(hash, &path);
        let output = table.to_string();

        assert!(output.contains("ROM Hash"));
        assert!(output.contains(hash));
        assert!(output.contains("Output Path"));
        assert!(output.contains("test/path/snapshot.bin"));
        assert!(output.contains("Status"));
        assert!(output.contains("Success"));
    }
}
