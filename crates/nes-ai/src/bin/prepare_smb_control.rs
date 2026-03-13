use std::{env, fs, path::PathBuf};

use nes_ai::snapshot::{sha256_hex, write_snapshot_bundle};
use nes_core::{NesCore, tas::TasMovie};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = env::args().collect::<Vec<_>>();
    if args.len() != 4 {
        eprintln!("Usage: prepare_smb_control <rom_path> <bootstrap_tas_json> <output_snapshot>");
        std::process::exit(2);
    }

    let rom_path = PathBuf::from(&args[1]);
    let movie_path = PathBuf::from(&args[2]);
    let out_path = PathBuf::from(&args[3]);

    let rom = fs::read(&rom_path)?;
    let rom_hash = sha256_hex(&rom);
    let movie: TasMovie = serde_json::from_slice(&fs::read(movie_path)?)?;

    let mut core = NesCore::new();
    core.load_ines_rom(&rom)?;
    movie.replay(&mut core)?;

    write_snapshot_bundle(&out_path, &rom_hash, "smb-control-v1", &core.save_state())?;
    println!("{}", out_path.display());
    Ok(())
}
