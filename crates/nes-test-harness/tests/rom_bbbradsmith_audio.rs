use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use nes_core::AUDIO_CHUNK_SAMPLES;
use nes_test_harness::{
    apu_write_hash, audio_stats, bbbradsmith_audio_golden_dir_path,
    bbbradsmith_audio_suite_rom_paths, capture_audio_window, collect_apu_register_writes,
    compare_waveforms, detect_mapper_id, mapper_supported_by_core, read_pcm_i16le, waveform_hash,
};

const WRITE_TRACE_STEPS_PER_ROM: u32 = 750_000;
const AUDIO_WARMUP_FRAMES: u32 = 60;
const AUDIO_CAPTURE_FRAMES: u32 = 180;
const AUDIBLE_RMS_FLOOR: f64 = 32.0;
const GOLDEN_FFT_SIZE: usize = 1_024;
const GOLDEN_MIN_CORRELATION: f64 = 0.92;
const GOLDEN_MIN_RMS_RATIO: f64 = 0.75;
const GOLDEN_MAX_RMS_RATIO: f64 = 1.33;
const GOLDEN_MAX_FFT_DB_DIFF: f64 = 7.5;

struct ApuWriteTraceSummary {
    writes_count: usize,
    unique_registers: BTreeSet<u16>,
    hash: u64,
}

#[test]
#[ignore = "requires roms.bbbradsmith_audio_suite_dir in nes.toml"]
fn bbbradsmith_audio_suite_write_trace_is_deterministic_per_rom() {
    let rom_paths = bbbradsmith_audio_suite_rom_paths();
    if rom_paths.is_empty() { return; }
    let mut tested_roms = 0_usize;
    let mut skipped = Vec::new();
    for rom_path in rom_paths {
        let bytes =
            fs::read(&rom_path).unwrap_or_else(|err| panic!("failed to read '{rom_path}': {err}"));
        let rom_name = rom_name(&rom_path);
        let mapper_id = detect_mapper_id(&bytes).unwrap_or(u16::MAX);
        if !mapper_supported_by_core(mapper_id) {
            skipped.push(format!("{rom_name} (mapper {mapper_id})"));
            continue;
        }
        tested_roms = tested_roms.saturating_add(1);
        let first = run_apu_write_trace_summary(&bytes);
        let second = run_apu_write_trace_summary(&bytes);

        assert!(
            first.writes_count > 0,
            "expected APU register writes from {rom_name}, got none"
        );
        assert!(
            !first.unique_registers.is_empty(),
            "expected at least one touched APU register from {rom_name}"
        );
        assert_eq!(
            first.writes_count, second.writes_count,
            "APU write count drifted across identical runs for {rom_name}"
        );
        assert_eq!(
            first.hash, second.hash,
            "APU write trace hash drifted across identical runs for {rom_name}"
        );
    }
    assert!(
        tested_roms > 0,
        "no bbbradsmith ROMs with supported mappers were found (supported: 0/1/2/4). skipped: {skipped:?}"
    );
}

#[test]
#[ignore = "requires roms.bbbradsmith_audio_suite_dir in nes.toml"]
fn bbbradsmith_audio_suite_audio_windows_are_deterministic_and_well_formed() {
    let rom_paths = bbbradsmith_audio_suite_rom_paths();
    if rom_paths.is_empty() { return; }
    let expected_samples = AUDIO_CAPTURE_FRAMES as usize * AUDIO_CHUNK_SAMPLES;
    let mut audible_roms = 0_usize;
    let mut tested_roms = 0_usize;
    let mut skipped = Vec::new();

    for rom_path in rom_paths {
        let bytes =
            fs::read(&rom_path).unwrap_or_else(|err| panic!("failed to read '{rom_path}': {err}"));
        let rom_name = rom_name(&rom_path);
        let mapper_id = detect_mapper_id(&bytes).unwrap_or(u16::MAX);
        if !mapper_supported_by_core(mapper_id) {
            skipped.push(format!("{rom_name} (mapper {mapper_id})"));
            continue;
        }
        tested_roms = tested_roms.saturating_add(1);
        let first_samples = run_audio_capture(&bytes);
        let second_samples = run_audio_capture(&bytes);
        let first_hash = waveform_hash(&first_samples);
        let second_hash = waveform_hash(&second_samples);
        let stats = audio_stats(&first_samples);

        assert_eq!(
            first_samples.len(),
            expected_samples,
            "unexpected audio sample count for {rom_name}"
        );
        assert_eq!(
            first_hash, second_hash,
            "audio waveform hash drifted across identical runs for {rom_name}"
        );
        assert!(
            stats.rms.is_finite() && stats.dc_offset.is_finite(),
            "non-finite audio metrics for {rom_name}: rms={} dc={}",
            stats.rms,
            stats.dc_offset
        );
        assert!(
            (0.0..=1.0).contains(&stats.clipping_ratio),
            "invalid clipping ratio for {rom_name}: {}",
            stats.clipping_ratio
        );
        if stats.rms >= AUDIBLE_RMS_FLOOR {
            audible_roms = audible_roms.saturating_add(1);
        }
    }

    assert!(
        tested_roms > 0,
        "no bbbradsmith ROMs with supported mappers were found (supported: 0/1/2/4). skipped: {skipped:?}"
    );
    assert!(
        audible_roms > 0,
        "suite produced no audible windows (rms >= {AUDIBLE_RMS_FLOOR})"
    );
}

#[test]
#[ignore = "requires roms.bbbradsmith_audio_suite_dir and roms.bbbradsmith_audio_golden_dir in nes.toml"]
fn bbbradsmith_audio_suite_matches_golden_pcm_tolerances() {
    let rom_paths = bbbradsmith_audio_suite_rom_paths();
    if rom_paths.is_empty() { return; }
    let golden_dir = bbbradsmith_audio_golden_dir_path();
    let mut checked_roms = 0_usize;
    let mut skipped = Vec::new();

    for rom_path in rom_paths {
        let bytes =
            fs::read(&rom_path).unwrap_or_else(|err| panic!("failed to read '{rom_path}': {err}"));
        let mapper_id = detect_mapper_id(&bytes).unwrap_or(u16::MAX);
        let rom_name = rom_name(&rom_path);
        if !mapper_supported_by_core(mapper_id) {
            skipped.push(format!("{rom_name} (mapper {mapper_id})"));
            continue;
        }

        let stem = Path::new(&rom_path)
            .file_stem()
            .and_then(|value| value.to_str())
            .unwrap_or_else(|| panic!("unable to derive file stem for {rom_name}"));
        let golden_path = Path::new(&golden_dir).join(format!("{stem}.s16le.pcm"));
        let golden_samples = read_pcm_i16le(&golden_path)
            .unwrap_or_else(|err| panic!("failed to load golden PCM for {rom_name}: {err}"));
        let captured_samples = run_audio_capture(&bytes);

        let comparison = compare_waveforms(&captured_samples, &golden_samples, GOLDEN_FFT_SIZE);
        assert!(
            comparison.correlation >= GOLDEN_MIN_CORRELATION,
            "golden mismatch for {rom_name}: low correlation {:.4} (min {:.4})",
            comparison.correlation,
            GOLDEN_MIN_CORRELATION
        );
        assert!(
            (GOLDEN_MIN_RMS_RATIO..=GOLDEN_MAX_RMS_RATIO).contains(&comparison.rms_ratio),
            "golden mismatch for {rom_name}: rms ratio {:.4} outside [{:.2}, {:.2}]",
            comparison.rms_ratio,
            GOLDEN_MIN_RMS_RATIO,
            GOLDEN_MAX_RMS_RATIO
        );
        assert!(
            comparison.fft_mean_abs_db_diff <= GOLDEN_MAX_FFT_DB_DIFF,
            "golden mismatch for {rom_name}: fft mean abs dB diff {:.4} > {:.2}",
            comparison.fft_mean_abs_db_diff,
            GOLDEN_MAX_FFT_DB_DIFF
        );
        checked_roms = checked_roms.saturating_add(1);
    }

    assert!(
        checked_roms > 0,
        "no bbbradsmith ROMs with supported mappers were compared against golden PCM. skipped: {skipped:?}"
    );
}

fn run_apu_write_trace_summary(rom_bytes: &[u8]) -> ApuWriteTraceSummary {
    let mut core = nes_core::NesCore::new();
    core.load_ines_rom(rom_bytes)
        .expect("failed to load bbbradsmith audio ROM");

    let writes =
        collect_apu_register_writes(&mut core, WRITE_TRACE_STEPS_PER_ROM).unwrap_or_else(|err| {
            panic!(
                "apu write trace collection failed, pc={:04X}, ppu_frame={}: {err}",
                core.cpu_pc(),
                core.ppu_frame_counter()
            )
        });
    let unique_registers = writes.iter().map(|event| event.addr).collect();
    ApuWriteTraceSummary {
        writes_count: writes.len(),
        unique_registers,
        hash: apu_write_hash(&writes),
    }
}

fn run_audio_capture(rom_bytes: &[u8]) -> Vec<i16> {
    capture_audio_window(rom_bytes, AUDIO_WARMUP_FRAMES, AUDIO_CAPTURE_FRAMES)
        .expect("audio capture should succeed")
}

fn rom_name(path: &str) -> String {
    Path::new(path)
        .file_name()
        .and_then(|name| name.to_str())
        .map(str::to_owned)
        .unwrap_or_else(|| path.to_owned())
}
