use std::env;
use std::path::PathBuf;

use nes_test_harness::{default_homebrew_rom_path, write_homebrew_rom};

fn main() {
    if let Err(err) = run() {
        eprintln!("{err}");
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
                    return Err("missing value after --out".to_owned());
                };
                out_path = PathBuf::from(path);
            }
            _ => return Err(format!("unknown argument '{arg}'")),
        }
    }

    write_homebrew_rom(&out_path)?;
    println!("Wrote homebrew ROM: {}", out_path.display());
    Ok(())
}
