//! bbbradsmith audio golden PCM capture utility
//!
//! Generates "golden" reference PCM audio files by running the bbbradsmith audio
//! test suite. These reference files are later used by the harness to ensure
//! emulator audio output does not regress.

use std::env;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use std::borrow::Cow;

use comfy_table::{Cell, Color as TableColor, Table, presets::UTF8_FULL};
use crossterm::style::{Color, Stylize};
use nes_config::{NesConfig, parse_config_path_arg};
use nes_test_harness::{
    AudioStats, audio_stats, capture_audio_window, detect_mapper_id, mapper_supported_by_core,
    waveform_hash, write_pcm_i16le,
};

const AUDIO_WARMUP_FRAMES: u32 = 60;
const AUDIO_CAPTURE_FRAMES: u32 = 180;

/// Represents data for a single row in the summary table.
/// We use `Cow<'static, str>` instead of `String` for text fields to eliminate
/// heap allocations for static values (like `"-"` or `"Written"`) while still
/// allowing dynamic string conversions (like formatted numbers) without complex lifetimes.
struct RowData {
    rom_name: String,
    status: Cow<'static, str>,
    status_color: TableColor,
    mapper: String,
    samples: Cow<'static, str>,
    rms: Cow<'static, str>,
    peak: Cow<'static, str>,
    hash: Cow<'static, str>,
}

fn format_skipped_mapper_row(rom_name: String, mapper_id: u16) -> RowData {
    RowData {
        rom_name,
        status: "Skip (Mapper)".into(),
        status_color: TableColor::Magenta,
        mapper: mapper_id.to_string(),
        samples: "-".into(),
        rms: "-".into(),
        peak: "-".into(),
        hash: "-".into(),
    }
}

fn format_skipped_existing_row(rom_name: String, mapper_id: u16) -> RowData {
    RowData {
        rom_name,
        status: "Skip (Exists)".into(),
        status_color: TableColor::Yellow,
        mapper: mapper_id.to_string(),
        samples: "-".into(),
        rms: "-".into(),
        peak: "-".into(),
        hash: "-".into(),
    }
}

fn format_written_row(
    rom_name: String,
    mapper_id: u16,
    samples_len: usize,
    stats: AudioStats,
    hash: u64,
) -> RowData {
    RowData {
        rom_name,
        status: "Written".into(),
        status_color: TableColor::Green,
        mapper: mapper_id.to_string(),
        samples: samples_len.to_string().into(),
        rms: format!("{:.2}", stats.rms).into(),
        peak: stats.peak.to_string().into(),
        hash: format!("{:016X}", hash).into(),
    }
}

fn print_processing_progress(stdout: &mut impl Write, rom_name: &str, color: Color) {
    let _ = write!(
        stdout,
        "\r\x1B[2K{}",
        format!("Processing {}...", rom_name).with(color)
    );
    let _ = stdout.flush();
}

fn main() {
    let mut stdout = std::io::stdout();
    if let Err(err) = run(&mut stdout) {
        eprintln!("\n{}", err);
        std::process::exit(1);
    }
}

fn run(stdout: &mut impl Write) -> Result<(), String> {
    let raw_args = env::args().skip(1).collect::<Vec<_>>();
    if raw_args.iter().any(|arg| arg == "--help" || arg == "-h") {
        println!("Usage: bbbradsmith_golden_capture [--config <path>] [--force]");
        std::process::exit(0);
    }

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

    println!(
        "{}",
        "Capturing bbbradsmith audio goldens..."
            .with(Color::Cyan)
            .bold()
    );

    let mut written = 0_usize;
    let mut skipped_mapper = 0_usize;
    let mut skipped_existing = 0_usize;
    let mut rows: Vec<RowData> = Vec::with_capacity(rom_paths.len());

    for rom_path in rom_paths {
        let rom_name = rom_path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("<unknown>")
            .to_owned();
        let rom_bytes = fs::read(&rom_path)
            .map_err(|err| format!("failed to read ROM '{}': {err}", rom_path.display()))?;
        let mapper_id = detect_mapper_id(&rom_bytes).unwrap_or(u16::MAX);
        if !mapper_supported_by_core(mapper_id) {
            print_processing_progress(stdout, &rom_name, Color::Yellow);
            rows.push(format_skipped_mapper_row(rom_name, mapper_id));
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
            print_processing_progress(stdout, &rom_name, Color::Yellow);
            rows.push(format_skipped_existing_row(rom_name, mapper_id));
            skipped_existing = skipped_existing.saturating_add(1);
            continue;
        }

        print_processing_progress(stdout, &rom_name, Color::Green);

        let samples = capture_audio_window(&rom_bytes, AUDIO_WARMUP_FRAMES, AUDIO_CAPTURE_FRAMES)?;
        write_pcm_i16le(&output_path, &samples)?;
        let stats = audio_stats(&samples);
        let hash = waveform_hash(&samples);

        rows.push(format_written_row(
            rom_name,
            mapper_id,
            samples.len(),
            stats,
            hash,
        ));

        written = written.saturating_add(1);
    }
    let _ = write!(stdout, "\r\x1B[2K");
    let _ = stdout.flush();

    let _ = writeln!(stdout, "\n{}", build_summary_table(rows));
    let _ = writeln!(
        stdout,
        "done: written={} skipped_mapper={} skipped_existing={}",
        written.to_string().with(Color::Green),
        skipped_mapper.to_string().with(Color::Magenta),
        skipped_existing.to_string().with(Color::Yellow),
    );

    if written == 0 && skipped_existing == 0 {
        return Err("no golden files were written".to_owned());
    }
    Ok(())
}

fn build_summary_table(rows: Vec<RowData>) -> Table {
    let mut table = Table::new();
    table.load_preset(UTF8_FULL).apply_modifier(comfy_table::modifiers::UTF8_ROUND_CORNERS).apply_modifier(comfy_table::modifiers::UTF8_SOLID_INNER_BORDERS);
    table.set_header(vec![
        Cell::new("ROM").fg(TableColor::Cyan),
        Cell::new("Status").fg(TableColor::Cyan),
        Cell::new("Mapper").fg(TableColor::Cyan),
        Cell::new("Samples").fg(TableColor::Cyan),
        Cell::new("RMS").fg(TableColor::Cyan),
        Cell::new("Peak").fg(TableColor::Cyan),
        Cell::new("Hash").fg(TableColor::Cyan),
    ]);

    for row in rows.into_iter() {
        table.add_row(vec![
            Cell::new(row.rom_name).fg(TableColor::Green),
            Cell::new(row.status).fg(row.status_color),
            Cell::new(row.mapper).fg(TableColor::Yellow),
            Cell::new(row.samples).fg(TableColor::Yellow),
            Cell::new(row.rms).fg(TableColor::Yellow),
            Cell::new(row.peak).fg(TableColor::Yellow),
            Cell::new(row.hash).fg(TableColor::Green),
        ]);
    }

    table
}

fn load_config(path: Option<&Path>) -> Result<NesConfig, String> {
    match path {
        Some(config_path) => NesConfig::load(config_path),
        None => NesConfig::load_or_default(None),
    }
}

fn collect_suite_roms(suite_dir: &Path) -> Result<Vec<PathBuf>, String> {
    let mut roms = Vec::with_capacity(32);
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

#[cfg(test)]
mod tests {
    use super::{
        RowData, build_summary_table, format_skipped_existing_row, format_skipped_mapper_row,
        format_written_row, print_processing_progress,
    };
    use comfy_table::Color as TableColor;
    use crossterm::style::Color;
    use nes_test_harness::AudioStats;

    #[test]
    fn print_processing_progress_writes_to_stdout_with_ansi() {
        let mut buf = Vec::new();
        print_processing_progress(&mut buf, "test_rom.nes", Color::Green);
        let output = String::from_utf8(buf).expect("should output valid utf8");
        assert!(output.contains("\r\x1B[2K"));
        assert!(output.contains("test_rom.nes"));
    }

    #[test]
    fn load_config_returns_default_when_no_path_provided() {
        use super::load_config;
        let result = load_config(None);
        assert!(result.is_ok());
    }

    #[test]
    fn format_skipped_mapper_row_returns_correct_data() {
        let row = format_skipped_mapper_row("test.nes".to_string(), 99);
        assert_eq!(row.rom_name, "test.nes");
        assert_eq!(row.status, "Skip (Mapper)");
        assert_eq!(row.mapper, "99");
        assert_eq!(row.samples, "-");
    }

    #[test]
    fn format_skipped_existing_row_returns_correct_data() {
        let row = format_skipped_existing_row("test2.nes".to_string(), 1);
        assert_eq!(row.rom_name, "test2.nes");
        assert_eq!(row.status, "Skip (Exists)");
        assert_eq!(row.mapper, "1");
        assert_eq!(row.rms, "-");
    }

    #[test]
    fn format_written_row_returns_correct_data() {
        let stats = AudioStats {
            rms: 12.345,
            peak: 100,
            dc_offset: 0.0,
            sample_count: 500,
            clipping_ratio: 0.0,
        };
        let row = format_written_row("test3.nes".to_string(), 4, 500, stats, 0x1234567890ABCDEF);
        assert_eq!(row.rom_name, "test3.nes");
        assert_eq!(row.status, "Written");
        assert_eq!(row.mapper, "4");
        assert_eq!(row.samples, "500");
        assert_eq!(row.rms, "12.35");
        assert_eq!(row.peak, "100");
        assert_eq!(row.hash, "1234567890ABCDEF");
    }

    #[test]
    fn build_summary_table_includes_all_columns() {
        let rows = vec![
            RowData {
                rom_name: "test1.nes".to_string(),
                status: "Written".into(),
                status_color: TableColor::Green,
                mapper: "0".to_string(),
                samples: "1000".into(),
                rms: "1.23".into(),
                peak: "50".into(),
                hash: "0000111122223333".into(),
            },
            RowData {
                rom_name: "test2.nes".to_string(),
                status: "Skip (Mapper)".into(),
                status_color: TableColor::Magenta,
                mapper: "255".to_string(),
                samples: "-".into(),
                rms: "-".into(),
                peak: "-".into(),
                hash: "-".into(),
            },
        ];

        let table = build_summary_table(rows);
        let output = table.to_string();

        assert!(output.contains("test1.nes"));
        assert!(output.contains("Written"));
        assert!(output.contains("1.23"));
        assert!(output.contains("0000111122223333"));

        assert!(output.contains("test2.nes"));
        assert!(output.contains("Skip (Mapper)"));
        assert!(output.contains("255"));
    }
}
