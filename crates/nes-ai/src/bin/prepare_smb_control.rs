use std::{env, fs, path::PathBuf};

use comfy_table::{Cell, Color as TableColor, Table};
use crossterm::style::{Color, Stylize};
use nes_ai::snapshot::{sha256_hex, write_snapshot_bundle};
use nes_core::{NesCore, tas::TasMovie};

fn main() {
    if let Err(err) = run() {
        eprintln!("\n{}", format!("Error: {err}").with(Color::Red).bold());
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let args = env::args().collect::<Vec<_>>();
    if args.len() != 4 {
        return Err(
            "Usage: prepare_smb_control <rom_path> <bootstrap_tas_json> <output_snapshot>"
                .to_owned(),
        );
    }

    let rom_path = PathBuf::from(&args[1]);
    let movie_path = PathBuf::from(&args[2]);
    let out_path = PathBuf::from(&args[3]);

    let rom = fs::read(&rom_path).map_err(|e| format!("Failed to read ROM: {e}"))?;
    let rom_hash = sha256_hex(&rom);

    let movie_json = fs::read(&movie_path).map_err(|e| format!("Failed to read TAS json: {e}"))?;
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
    table.set_header(vec![
        Cell::new("Metric").fg(TableColor::Cyan),
        Cell::new("Value").fg(TableColor::White),
    ]);

    table.add_row(vec![
        Cell::new("ROM Hash"),
        Cell::new(rom_hash).fg(TableColor::Yellow),
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
