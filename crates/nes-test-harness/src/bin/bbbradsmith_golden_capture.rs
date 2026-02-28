use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use nes_config::{NesConfig, parse_config_path_arg};
use nes_test_harness::{
    audio_stats, capture_audio_window, detect_mapper_id, mapper_supported_by_core, waveform_hash,
    write_pcm_i16le,
};

const AUDIO_WARMUP_FRAMES: u32 = 60;
const AUDIO_CAPTURE_FRAMES: u32 = 180;

fn main() {
    if let Err(err) = run() {
        eprintln!("{err}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let raw_args = env::args().skip(1).collect::<Vec<_>>();
    let (config_path, pass_through) = parse_config_path_arg(&raw_args)?;
    let force = pass_through.iter().any(|arg| arg == "--force");
    for arg in &pass_through {
        if arg != "--force" {
            return Err(format!(
                "unknown argument '{arg}'. supported: --config <path>, --config=<path>, --force"
            ));
        }
    }

    let config = load_config(config_path.as_deref())?;
    let suite_dir = config.roms.bbbradsmith_audio_suite_dir.ok_or_else(|| {
        "missing `roms.bbbradsmith_audio_suite_dir` in config for input ROM suite".to_owned()
    })?;
    let golden_dir = config.roms.bbbradsmith_audio_golden_dir.ok_or_else(|| {
        "missing `roms.bbbradsmith_audio_golden_dir` in config for golden PCM output".to_owned()
    })?;

    let suite_dir_path = Path::new(&suite_dir);
    if !suite_dir_path.is_dir() {
        return Err(format!(
            "bbbradsmith audio suite directory does not exist or is not a directory: {}",
            suite_dir_path.display()
        ));
    }
    fs::create_dir_all(&golden_dir)
        .map_err(|err| format!("failed to create golden directory '{golden_dir}': {err}"))?;

    let mut rom_paths = collect_suite_roms(suite_dir_path)?;
    rom_paths.sort_unstable_by_key(|path| path.to_string_lossy().to_ascii_lowercase());
    if rom_paths.is_empty() {
        return Err(format!(
            "no .nes files found in suite directory {}",
            suite_dir_path.display()
        ));
    }

    let mut written = 0_usize;
    let mut skipped_mapper = 0_usize;
    let mut skipped_existing = 0_usize;

    for rom_path in rom_paths {
        let rom_name = rom_path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("<unknown>");
        let rom_bytes = fs::read(&rom_path)
            .map_err(|err| format!("failed to read ROM '{}': {err}", rom_path.display()))?;
        let mapper_id = detect_mapper_id(&rom_bytes).unwrap_or(u16::MAX);
        if !mapper_supported_by_core(mapper_id) {
            println!("skip {rom_name}: unsupported mapper {mapper_id}");
            skipped_mapper = skipped_mapper.saturating_add(1);
            continue;
        }

        let stem = rom_path
            .file_stem()
            .and_then(|name| name.to_str())
            .ok_or_else(|| {
                format!(
                    "unable to compute output stem from ROM path '{}'",
                    rom_path.display()
                )
            })?;
        let output_path = PathBuf::from(&golden_dir).join(format!("{stem}.s16le.pcm"));
        if output_path.exists() && !force {
            println!(
                "skip {rom_name}: golden already exists ({})",
                output_path.display()
            );
            skipped_existing = skipped_existing.saturating_add(1);
            continue;
        }

        let samples = capture_audio_window(&rom_bytes, AUDIO_WARMUP_FRAMES, AUDIO_CAPTURE_FRAMES)?;
        write_pcm_i16le(&output_path, &samples)?;
        let stats = audio_stats(&samples);
        println!(
            "write {rom_name}: mapper={mapper_id} samples={} rms={:.2} peak={} hash={:016X} -> {}",
            samples.len(),
            stats.rms,
            stats.peak,
            waveform_hash(&samples),
            output_path.display()
        );
        written = written.saturating_add(1);
    }

    println!(
        "done: written={written} skipped_mapper={skipped_mapper} skipped_existing={skipped_existing}"
    );
    if written == 0 && skipped_existing == 0 {
        return Err("no golden files were written".to_owned());
    }
    Ok(())
}

fn load_config(path: Option<&Path>) -> Result<NesConfig, String> {
    match path {
        Some(config_path) => NesConfig::load(config_path),
        None => NesConfig::load_or_default(None),
    }
}

fn collect_suite_roms(suite_dir: &Path) -> Result<Vec<PathBuf>, String> {
    let mut roms = Vec::new();
    for entry in fs::read_dir(suite_dir).map_err(|err| {
        format!(
            "failed to read suite directory '{}': {err}",
            suite_dir.display()
        )
    })? {
        let entry = entry.map_err(|err| {
            format!(
                "failed to inspect directory entry in '{}': {err}",
                suite_dir.display()
            )
        })?;
        let path = entry.path();
        if path.is_file()
            && path
                .extension()
                .is_some_and(|ext| ext.eq_ignore_ascii_case("nes"))
        {
            roms.push(path);
        }
    }
    Ok(roms)
}
