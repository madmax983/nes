//! Reusable testing utilities and integration harnesses for the NES emulator.
//!
//! When developing an emulator, unit tests only verify the smallest components.
//! True confidence comes from integration testing—feeding a known ROM into the `NesCore`,
//! executing it for several thousand cycles, and verifying the resulting hardware state.
//!
//! This crate prevents duplication across the workspace by providing tools to:
//! - Build and inject minimal homebrew ROMs.
//! - Trace APU register writes to ensure cycle-accurate sound events.
//! - Capture and analyze `.pcm` audio streams against "golden" known-good recordings.
//!
//! ## Examples
//!
//! Using the harness to collect audio states:
//!
//! ```rust
//! use nes_test_harness::{AudioStats, audio_stats};
//!
//! let sine_wave = [0, 16000, 32000, 16000, 0, -16000, -32000, -16000];
//! let stats = audio_stats(&sine_wave);
//! assert_eq!(stats.sample_count, 8);
//! assert_eq!(stats.peak, 32000);
//! ```
use std::f64::consts::PI;
use std::fs;
use std::path::Path;

mod homebrew;
/// Provides hardcoded path resolution for testing ROMs.
pub mod rom_paths;

pub use homebrew::{build_homebrew_rom, default_homebrew_rom_path, write_homebrew_rom};
pub use rom_paths::*;

use nes_core::{Command, CoreError, NesCore, cpu::CpuBusAccessKind};

/// A flight recorder event for the Audio Processing Unit.
///
/// When writing cycle-accurate emulators, you cannot simply listen to the audio output to verify correctness.
/// Instead, you must trap the exact CPU cycles where the game alters the APU registers and compare this
/// trace against a known-good emulation. This struct captures a single frame-perfect hardware mutation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ApuWriteEvent {
    /// The absolute, continuous CPU cycle since power-on. This defines the exact moment in time the APU state mutated.
    pub cpu_cycle: u64,
    /// The memory-mapped I/O address hit by the CPU (typically in the `0x4000` to `0x4017` range).
    pub addr: u16,
    /// The raw byte blasted into the register, dictating duty cycles, sweeps, envelopes, or period limits.
    pub value: u8,
}

/// A statistical profile of a captured audio window.
///
/// Sometimes tests do not demand cycle-perfect hashes. If a test just needs to ensure a square wave
/// is *loud* or that silence has no DC offset, this struct provides the macroscopic physical characteristics
/// of the output waveform without drowning in the micro-details of individual samples.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AudioStats {
    /// The total volume of discrete samples ingested to calculate these statistics.
    pub sample_count: usize,
    /// Root Mean Square. Represents the continuous power (perceived volume) of the signal.
    pub rms: f64,
    /// The absolute maximum excursion of the waveform from zero. Useful for detecting clipping thresholds.
    pub peak: i16,
    /// The arithmetic mean. A perfect AC signal averages to `0.0`. A high offset implies a silent channel
    /// was left held high, which can cause violent "pops" when mixed into hardware.
    pub dc_offset: f64,
    /// The percentage of samples living dangerously close to the `i16` ceiling (`32,760`). If this is high,
    /// your mixer is digitally distorting and sounds terrible.
    pub clipping_ratio: f64,
}

/// The scientific verdict comparing an emulator's audio output against a golden "perfect" recording.
///
/// When asserting that an audio change hasn't broken things, humans are bad judges. This struct uses
/// time-domain (Pearson correlation) and frequency-domain (FFT) analysis to prove mathematically that
/// the emulator sounds identical to the hardware, ignoring negligible floating-point rounding errors.
#[derive(Debug, Clone, PartialEq)]
pub struct WaveformComparison {
    /// How many samples were aligned and cross-examined.
    pub samples_compared: usize,
    /// Time-domain similarity. `1.0` means they trace the exact same shape. `0.0` is pure noise. `-1.0` is inverted.
    pub correlation: f64,
    /// Power similarity. If `1.0`, both are equally loud. If `0.5`, the emulator is outputting at half volume.
    pub rms_ratio: f64,
    /// Frequency-domain similarity. The average difference in Decibels across all frequency bins.
    /// Low numbers mean the "timbre" and "pitch" match perfectly, even if the wave is phase-shifted.
    pub fft_mean_abs_db_diff: f64,
}

const INES_HEADER_LEN: usize = 16;
const INES_MAGIC: [u8; 4] = [0x4E, 0x45, 0x53, 0x1A];

/// Steps the core and records all writes made to the APU registers (`0x4000`..=`0x4017`).
///
/// This is heavily used in `bbbradsmith_golden_capture` style tests. By inspecting the sequence
/// and exact cycle timestamps of these writes, we can prove the CPU execution timing is flawless.
///
/// ## Examples
///
/// ```rust
/// use nes_core::{NesCore, Command};
/// use nes_test_harness::collect_apu_register_writes;
///
/// let mut core = NesCore::new();
/// // In a real test, load a test ROM here.
/// let writes = collect_apu_register_writes(&mut core, 10).unwrap();
/// ```
pub fn collect_apu_register_writes(
    core: &mut NesCore,
    cpu_steps: u32,
) -> Result<Vec<ApuWriteEvent>, CoreError> {
    let mut writes = Vec::with_capacity(cpu_steps as usize / 16);
    for _ in 0..cpu_steps {
        core.execute(Command::StepCpu)?;
        let cpu_cycle = core.total_cycles();
        for access in core.last_cpu_bus_trace() {
            if access.kind == CpuBusAccessKind::Write && (0x4000..=0x4017).contains(&access.addr) {
                writes.push(ApuWriteEvent {
                    cpu_cycle,
                    addr: access.addr,
                    value: access.value,
                });
            }
        }
    }
    Ok(writes)
}

/// Crushes a massive sequence of APU mutations into a single cryptographic-style snapshot.
///
/// This allows tests to execute millions of CPU cycles and verify the exact timing of thousands of audio
/// events using a single assertion against a hardcoded `u64` hash.
///
/// ## Examples
///
/// ```rust
/// use nes_test_harness::{ApuWriteEvent, apu_write_hash};
///
/// let events = [ApuWriteEvent { cpu_cycle: 1, addr: 0x4000, value: 0xFF }];
/// assert_ne!(apu_write_hash(&events), 0);
/// ```
#[must_use]
pub fn apu_write_hash(writes: &[ApuWriteEvent]) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325_u64;
    for event in writes {
        hash ^= event.cpu_cycle;
        hash = hash.wrapping_mul(0x0000_0001_0000_01b3);
        hash ^= u64::from(event.addr);
        hash = hash.wrapping_mul(0x0000_0001_0000_01b3);
        hash ^= u64::from(event.value);
        hash = hash.wrapping_mul(0x0000_0001_0000_01b3);
    }
    hash
}

/// Steps the given [`NesCore`] for a specific number of frames and collects the generated audio samples.
///
/// This is useful for writing test assertions against audio output (e.g., verifying a square wave).
/// Ensure that the [`NesCore`] has a valid ROM loaded prior to calling this; stepping an empty
/// core will result in silent samples.
///
/// # Examples
///
/// ```
/// use nes_core::{NesCore, Command};
/// use nes_test_harness::collect_audio_for_frames;
///
/// let mut core = NesCore::new();
/// // In a real test, load a ROM here before stepping
/// let audio = collect_audio_for_frames(&mut core, 2).unwrap();
/// assert_eq!(audio.len(), nes_core::AUDIO_CHUNK_SAMPLES * 2);
/// ```
pub fn collect_audio_for_frames(core: &mut NesCore, frames: u32) -> Result<Vec<i16>, CoreError> {
    let mut samples = Vec::with_capacity((frames as usize) * nes_core::AUDIO_CHUNK_SAMPLES);
    for _ in 0..frames {
        core.execute(Command::StepFrame)?;
        samples.extend(core.audio_chunk_i16());
    }
    Ok(samples)
}

/// Loads a ROM, advances past an initial warmup period, and captures a window of audio samples.
///
/// This helper handles the full lifecycle for audio testing, including ROM loading and skipping
/// the initial frames (where the game is booting or silent) to capture the target sound.
///
/// # Errors
///
/// Returns an error if the ROM is invalid or if the core fails to step during execution.
///
/// # Examples
///
/// ```
/// use nes_test_harness::{capture_audio_window, build_homebrew_rom};
///
/// let rom_bytes = build_homebrew_rom().unwrap();
/// // Skip the first 10 frames, then capture 5 frames of audio
/// let audio = capture_audio_window(&rom_bytes, 10, 5).unwrap();
/// assert_eq!(audio.len(), nes_core::AUDIO_CHUNK_SAMPLES * 5);
/// ```
pub fn capture_audio_window(
    rom_bytes: &[u8],
    warmup_frames: u32,
    capture_frames: u32,
) -> Result<Vec<i16>, String> {
    let mut core = NesCore::new();
    core.load_ines_rom(rom_bytes)
        .map_err(|err| format!("failed to load ROM for audio capture: {err}"))?;

    for frame in 0..warmup_frames {
        core.execute(Command::StepFrame).map_err(|err| {
            format!(
                "audio warmup frame step failed at frame {frame}, pc={:04X}: {err}",
                core.cpu_pc()
            )
        })?;
    }
    collect_audio_for_frames(&mut core, capture_frames)
        .map_err(|err| format!("audio capture failed after warmup: {err}"))
}

/// Hashes raw PCM audio for strict, bit-for-bit regression snapshot testing.
///
/// Only use this when you guarantee deterministic rendering (e.g. no asynchronous host sample rate conversion).
///
/// ## Examples
///
/// ```rust
/// use nes_test_harness::waveform_hash;
/// assert_ne!(waveform_hash(&[0, 10, -10, 0]), 0);
/// ```
#[must_use]
pub fn waveform_hash(samples: &[i16]) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325_u64;
    for sample in samples {
        hash ^= u64::from(*sample as u16);
        hash = hash.wrapping_mul(0x0000_0001_0000_01b3);
    }
    hash
}

/// Sweeps a raw PCM buffer and extracts its physical profile (power, peaks, DC offset).
///
/// Useful for making intelligent assertions like "This waveform should be perfectly silent" without
/// writing `assert_eq!(sample, 0)` ten thousand times.
///
/// ## Examples
///
/// ```rust
/// use nes_test_harness::audio_stats;
/// let stats = audio_stats(&[100, -100]);
/// assert_eq!(stats.peak, 100);
/// assert_eq!(stats.dc_offset, 0.0);
/// ```
#[must_use]
pub fn audio_stats(samples: &[i16]) -> AudioStats {
    if samples.is_empty() {
        return AudioStats {
            sample_count: 0,
            rms: 0.0,
            peak: 0,
            dc_offset: 0.0,
            clipping_ratio: 0.0,
        };
    }

    let mut sum = 0.0_f64;
    let mut sum_sq = 0.0_f64;
    let mut peak = 0_i32;
    let mut clipping_count = 0_usize;

    for sample in samples {
        let value = f64::from(*sample);
        sum += value;
        sum_sq += value * value;
        let magnitude = i32::from(*sample).abs();
        peak = peak.max(magnitude);
        if magnitude >= 32_760 {
            clipping_count = clipping_count.saturating_add(1);
        }
    }

    let n = samples.len() as f64;
    AudioStats {
        sample_count: samples.len(),
        rms: (sum_sq / n).sqrt(),
        peak: peak as i16,
        dc_offset: sum / n,
        clipping_ratio: clipping_count as f64 / n,
    }
}

/// Slices a massive audio stream into chunks and measures the power (RMS) of each chunk.
///
/// This allows a test to verify the "envelope" of a sound—proving that a note starts loud and
/// decays exponentially, rather than analyzing individual high-frequency oscillations.
///
/// ## Examples
///
/// ```rust
/// use nes_test_harness::rms_envelope;
/// let decay = rms_envelope(&[100, 100, 10, 10, 0, 0], 2);
/// assert!(decay[0] > decay[1]);
/// ```
#[must_use]
pub fn rms_envelope(samples: &[i16], window_samples: usize) -> Vec<f64> {
    if samples.is_empty() || window_samples == 0 {
        return Vec::new();
    }

    let mut envelope = Vec::with_capacity(samples.len().div_ceil(window_samples));
    for chunk in samples.chunks(window_samples) {
        let mut sum_sq = 0.0_f64;
        for sample in chunk {
            let value = f64::from(*sample);
            sum_sq += value * value;
        }
        let n = chunk.len() as f64;
        envelope.push((sum_sq / n).sqrt());
    }
    envelope
}

/// Calculates the Pearson correlation coefficient between two audio waveforms.
/// Avoids unnecessary heap allocations by converting `i16` to `f64` on the fly.
#[must_use]
pub fn pearson_correlation(lhs: &[i16], rhs: &[i16]) -> f64 {
    let n = lhs.len().min(rhs.len());
    if n < 2 {
        return 0.0;
    }

    let lhs = &lhs[..n];
    let rhs = &rhs[..n];

    let lhs_mean = lhs.iter().map(|&v| f64::from(v)).sum::<f64>() / n as f64;
    let rhs_mean = rhs.iter().map(|&v| f64::from(v)).sum::<f64>() / n as f64;

    let mut numerator = 0.0_f64;
    let mut lhs_var = 0.0_f64;
    let mut rhs_var = 0.0_f64;
    for (left, right) in lhs.iter().zip(rhs.iter()) {
        let left_centered = f64::from(*left) - lhs_mean;
        let right_centered = f64::from(*right) - rhs_mean;
        numerator += left_centered * right_centered;
        lhs_var += left_centered * left_centered;
        rhs_var += right_centered * right_centered;
    }

    let denom = (lhs_var * rhs_var).sqrt();
    if denom <= f64::EPSILON {
        return 0.0;
    }
    (numerator / denom).clamp(-1.0, 1.0)
}

/// Serializes raw 16-bit PCM audio samples to disk for offline analysis.
///
/// These files can be opened in Audacity (Import -> Raw Data, Signed 16-bit PCM, Little-Endian, 1 Channel).
///
/// ## Examples
///
/// ```rust
/// use std::path::Path;
/// use nes_test_harness::write_pcm_i16le;
///
/// // In tests, use a temp directory.
/// // write_pcm_i16le(Path::new("test_output.pcm"), &[0, 100, -100, 0]).unwrap();
/// ```
pub fn write_pcm_i16le(path: &Path, samples: &[i16]) -> Result<(), String> {
    let mut bytes = Vec::with_capacity(samples.len() * 2);
    for sample in samples {
        bytes.extend_from_slice(&sample.to_le_bytes());
    }
    fs::write(path, bytes).map_err(|err| format!("failed to write '{}': {err}", path.display()))
}

/// Deserializes a raw 16-bit PCM audio file from disk into a vector of samples.
///
/// Typically used to load a "golden" output stream to compare against generated emulator output.
///
/// ## Examples
///
/// ```rust
/// use std::path::Path;
/// use nes_test_harness::read_pcm_i16le;
///
/// // let samples = read_pcm_i16le(Path::new("golden_reference.pcm")).unwrap();
/// ```
pub fn read_pcm_i16le(path: &Path) -> Result<Vec<i16>, String> {
    let bytes =
        fs::read(path).map_err(|err| format!("failed to read '{}': {err}", path.display()))?;
    if !bytes.len().is_multiple_of(2) {
        return Err(format!(
            "invalid PCM byte length for '{}': expected even length, got {}",
            path.display(),
            bytes.len()
        ));
    }
    let mut samples = Vec::with_capacity(bytes.len() / 2);
    for chunk in bytes.chunks_exact(2) {
        samples.push(i16::from_le_bytes([chunk[0], chunk[1]]));
    }
    Ok(samples)
}

/// Executes a comprehensive forensic analysis between two audio streams.
///
/// If a test fails this comparison, your emulator has either dropped samples, drifted in pitch, or
/// violently altered its volume logic.
///
/// ## Examples
///
/// ```rust
/// use nes_test_harness::compare_waveforms;
/// let a = [0, 10, 0, -10];
/// let b = [0, 10, 0, -10];
/// let result = compare_waveforms(&a, &b, 4);
/// assert_eq!(result.correlation, 1.0);
/// ```
#[must_use]
pub fn compare_waveforms(lhs: &[i16], rhs: &[i16], fft_size: usize) -> WaveformComparison {
    let n = lhs.len().min(rhs.len());
    if n == 0 {
        return WaveformComparison {
            samples_compared: 0,
            correlation: 0.0,
            rms_ratio: 0.0,
            fft_mean_abs_db_diff: f64::INFINITY,
        };
    }

    let lhs = &lhs[..n];
    let rhs = &rhs[..n];

    let lhs_stats = audio_stats(lhs);
    let rhs_stats = audio_stats(rhs);
    let rms_ratio = if rhs_stats.rms <= f64::EPSILON {
        0.0
    } else {
        lhs_stats.rms / rhs_stats.rms
    };

    let lhs_fft = fft_log_mag_db(lhs, fft_size);
    let rhs_fft = fft_log_mag_db(rhs, fft_size);
    let bins = lhs_fft.len().min(rhs_fft.len());
    let fft_mean_abs_db_diff = if bins == 0 {
        f64::INFINITY
    } else {
        lhs_fft
            .iter()
            .zip(rhs_fft.iter())
            .map(|(left, right)| (left - right).abs())
            .sum::<f64>()
            / bins as f64
    };

    WaveformComparison {
        samples_compared: n,
        correlation: pearson_correlation(lhs, rhs),
        rms_ratio,
        fft_mean_abs_db_diff,
    }
}

/// Converts time-domain PCM samples into a frequency-domain spectrogram.
///
/// This proves whether the APU is emitting the correct *musical pitch* by measuring the decibel strength
/// of distinct frequency bins. Uses a naive O(N^2) Discrete Fourier Transform, so keep `fft_size` small in tests.
///
/// ## Examples
///
/// ```rust
/// use nes_test_harness::fft_log_mag_db;
/// let spectrum = fft_log_mag_db(&[0, 32000, 0, -32000], 4);
/// assert!(!spectrum.is_empty());
/// ```
#[must_use]
pub fn fft_log_mag_db(samples: &[i16], fft_size: usize) -> Vec<f64> {
    if samples.is_empty() || fft_size < 2 {
        return Vec::new();
    }

    // Intentionally small in tests to keep this O(N^2) DFT practical.
    let size = fft_size.min(samples.len()).next_power_of_two().max(2);
    let mut windowed = vec![0.0_f64; size];
    for idx in 0..size {
        let sample = if idx < samples.len() {
            f64::from(samples[idx])
        } else {
            0.0
        };
        windowed[idx] = sample * hann_window(idx, size);
    }

    let nyquist = size / 2;
    let mut bins = Vec::with_capacity(nyquist);
    for k in 0..nyquist {
        let mut re = 0.0_f64;
        let mut im = 0.0_f64;
        for (t, sample) in windowed.iter().enumerate() {
            let angle = (2.0 * PI * (k as f64) * (t as f64)) / size as f64;
            re += sample * angle.cos();
            im -= sample * angle.sin();
        }
        let mag = ((re * re + im * im).sqrt() / size as f64).max(1.0e-12);
        bins.push(20.0 * mag.log10());
    }
    bins
}

fn hann_window(idx: usize, len: usize) -> f64 {
    if len <= 1 {
        return 1.0;
    }
    let phase = (2.0 * PI * idx as f64) / (len - 1) as f64;
    0.5 - 0.5 * phase.cos()
}

/// Interrogates the raw binary header of an iNES ROM file to extract its Mapper ID.
///
/// The Mapper ID dictates the physical cartridge hardware the emulator must simulate (e.g. MMC3 vs UXROM).
/// This function protects the emulator from trying to execute a ROM it lacks the silicon for.
///
/// ## Examples
///
/// ```rust
/// use nes_test_harness::detect_mapper_id;
/// let mut rom = [0; 16];
/// rom[0..4].copy_from_slice(b"NES\x1A");
/// rom[6] = 0x40; // Mapper 4
/// assert_eq!(detect_mapper_id(&rom), Some(4));
/// ```
#[must_use]
pub fn detect_mapper_id(rom_bytes: &[u8]) -> Option<u16> {
    if rom_bytes.len() < INES_HEADER_LEN || rom_bytes[0..4] != INES_MAGIC {
        return None;
    }

    let flags6 = rom_bytes[6];
    let flags7 = rom_bytes[7];
    let is_nes2 = (flags7 & 0b0000_1100) == 0b0000_1000;
    let mapper_low = u16::from((flags6 >> 4) | (flags7 & 0xF0));
    if is_nes2 {
        Some(mapper_low | (u16::from(rom_bytes[8] & 0x0F) << 8))
    } else {
        Some(mapper_low)
    }
}

/// The whitelist of memory mappers currently implemented in the `nes-core` simulation.
///
/// Returns `true` if the emulator knows how to bank-switch this cartridge.
///
/// ## Examples
///
/// ```rust
/// use nes_test_harness::mapper_supported_by_core;
/// assert!(mapper_supported_by_core(0)); // NROM is always safe.
/// ```
#[must_use]
pub fn mapper_supported_by_core(mapper_id: u16) -> bool {
    matches!(mapper_id, 0 | 1 | 2 | 4)
}

#[cfg(test)]
mod tests {
    use super::{
        apu_write_hash, audio_stats, collect_apu_register_writes, compare_waveforms,
        detect_mapper_id, mapper_supported_by_core, pearson_correlation, read_pcm_i16le,
        rms_envelope, waveform_hash, write_pcm_i16le,
    };
    use nes_core::NesCore;

    #[test]
    fn waveform_hash_is_stable_for_known_input() {
        let samples = [1_i16, -2, 3, -4, 5, -6];
        assert_eq!(waveform_hash(&samples), waveform_hash(&samples));
    }

    #[test]
    fn audio_stats_reports_expected_geometry() {
        let samples = [10_i16, -10, 10, -10];
        let stats = audio_stats(&samples);
        assert_eq!(stats.sample_count, 4);
        assert_eq!(stats.peak, 10);
        assert_eq!(stats.dc_offset, 0.0);
        assert!(stats.rms > 0.0);
        assert_eq!(stats.clipping_ratio, 0.0);
    }

    #[test]
    fn pearson_correlation_is_one_for_identical_sequences() {
        let lhs = [1_i16, 2, 3, 4, 5];
        assert_eq!(pearson_correlation(&lhs, &lhs), 1.0);
    }

    #[test]
    fn rms_envelope_respects_windowing() {
        let samples = [3_i16, 4, 0, 0];
        let envelope = rms_envelope(&samples, 2);
        assert_eq!(envelope.len(), 2);
        assert!(envelope[0] > envelope[1]);
    }

    #[test]
    fn collect_apu_register_writes_tracks_apu_bus_stores() {
        let mut core = NesCore::new();
        core.load_cpu_bytes(
            0xC000,
            &[
                0xA9, 0x0F, // LDA #$0F
                0x8D, 0x00, 0x40, // STA $4000
                0x4C, 0x00, 0xC0, // JMP $C000
            ],
        );

        let writes =
            collect_apu_register_writes(&mut core, 8).expect("step cpu should not fail in loop");
        assert!(
            writes
                .iter()
                .any(|event| event.addr == 0x4000 && event.value == 0x0F),
            "expected write to $4000 in captured APU write trace"
        );

        let hash = apu_write_hash(&writes);
        assert_ne!(hash, 0);
        assert_ne!(hash, 1);

        let mut writes = writes;

        writes[0].cpu_cycle ^= 0xFFFF;
        assert_ne!(hash, apu_write_hash(&writes));
        writes[0].cpu_cycle ^= 0xFFFF;

        writes[0].addr ^= 0xFFFF;
        assert_ne!(hash, apu_write_hash(&writes));
        writes[0].addr ^= 0xFFFF;

        writes[0].value ^= 0xFF;
        assert_ne!(hash, apu_write_hash(&writes));
        writes[0].value ^= 0xFF;

        let orig_cycle = writes[0].cpu_cycle;
        writes[0].cpu_cycle |= 0xFFFF;
        if writes[0].cpu_cycle != orig_cycle {
            assert_ne!(hash, apu_write_hash(&writes));
        }
        writes[0].cpu_cycle = orig_cycle;

        let orig_addr = writes[0].addr;
        writes[0].addr &= 0x0000;
        if writes[0].addr != orig_addr {
            assert_ne!(hash, apu_write_hash(&writes));
        }
        writes[0].addr = orig_addr;

        let orig_value_or = writes[0].value;
        writes[0].value |= 0xFF;
        if writes[0].value != orig_value_or {
            assert_ne!(hash, apu_write_hash(&writes));
        }
        writes[0].value = orig_value_or;

        let orig_value_and = writes[0].value;
        writes[0].value &= 0x00;
        if writes[0].value != orig_value_and {
            assert_ne!(hash, apu_write_hash(&writes));
        }
        writes[0].value = orig_value_and;
    }

    #[test]
    fn collect_apu_register_writes_ignores_reads() {
        let mut core = NesCore::new();
        core.load_cpu_bytes(
            0xC000,
            &[
                0xAD, 0x00, 0x40, // LDA $4000
                0x4C, 0x00, 0xC0, // JMP $C000
            ],
        );

        let writes =
            collect_apu_register_writes(&mut core, 8).expect("step cpu should not fail in loop");
        assert!(writes.is_empty(), "expected reads to be ignored");
    }

    #[test]
    fn collect_apu_register_writes_ignores_non_apu_writes() {
        let mut core = NesCore::new();
        core.load_cpu_bytes(
            0xC000,
            &[
                0xA9, 0x0F, // LDA #$0F
                0x8D, 0x18, 0x40, // STA $4018 (outside APU range)
                0x4C, 0x00, 0xC0, // JMP $C000
            ],
        );

        let writes =
            collect_apu_register_writes(&mut core, 8).expect("step cpu should not fail in loop");
        assert!(
            writes.is_empty(),
            "expected out of bounds writes to be ignored"
        );
    }

    #[test]
    fn pcm_round_trip_preserves_samples() {
        let mut path = std::env::temp_dir();
        path.push(format!("nes_test_pcm_roundtrip_{}.pcm", std::process::id()));
        let samples = vec![-32768_i16, -123, 0, 123, 32767];
        write_pcm_i16le(&path, &samples).expect("pcm write should succeed");
        let loaded = read_pcm_i16le(&path).expect("pcm read should succeed");
        assert_eq!(samples, loaded);
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn compare_waveforms_reports_high_similarity_for_near_identical_streams() {
        let lhs = [0_i16, 100, 200, 100, 0, -100, -200, -100];
        let rhs = [1_i16, 101, 201, 101, 1, -99, -199, -99];
        let comparison = compare_waveforms(&lhs, &rhs, 8);
        assert!(comparison.correlation > 0.99);
        assert!(comparison.fft_mean_abs_db_diff < 1.0);
    }

    #[test]
    fn detect_mapper_id_reads_ines_header() {
        let mut rom = [0_u8; 16];
        rom[0..4].copy_from_slice(b"NES\x1A");
        rom[6] = 0x10;
        rom[7] = 0x20;
        assert_eq!(detect_mapper_id(&rom), Some(0x21));
    }

    #[test]
    fn mapper_supported_by_core_matches_core_surface() {
        assert!(mapper_supported_by_core(0));
        assert!(mapper_supported_by_core(1));
        assert!(mapper_supported_by_core(2));
        assert!(mapper_supported_by_core(4));
        assert!(!mapper_supported_by_core(69));
    }
}
