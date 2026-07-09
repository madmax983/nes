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

/// Represents a single write to an APU register during emulator execution.
///
/// This is used to trace exactly when and what audio events are triggered
/// by a ROM, allowing for cycle-accurate audio tests.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ApuWriteEvent {
    /// The absolute CPU cycle on which the write occurred.
    pub cpu_cycle: u64,
    /// The memory address of the APU register being written to (e.g., `0x4000`).
    pub addr: u16,
    /// The value written to the APU register.
    pub value: u8,
}

/// Statistics calculated from a raw audio PCM buffer.
///
/// These stats are useful for identifying silent streams, clipping issues, or
/// general audio levels without relying on a full waveform comparison.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AudioStats {
    /// The total number of samples analyzed.
    pub sample_count: usize,
    /// The Root Mean Square (RMS) volume level of the buffer.
    pub rms: f64,
    /// The absolute peak amplitude found in the buffer.
    pub peak: i16,
    /// The average DC offset (how far the waveform drifts from 0).
    pub dc_offset: f64,
    /// The ratio of samples that reached the maximum amplitude.
    pub clipping_ratio: f64,
}

/// The result of comparing two audio waveforms against each other.
///
/// This provides a comprehensive check to see if an emulated audio stream matches
/// a known-good "golden" standard.
#[derive(Debug, Clone, PartialEq)]
pub struct WaveformComparison {
    /// The number of samples that were compared (the shorter of the two streams).
    pub samples_compared: usize,
    /// The Pearson correlation coefficient (-1.0 to 1.0) indicating waveform similarity.
    pub correlation: f64,
    /// The ratio of the Root Mean Square levels between the two streams.
    pub rms_ratio: f64,
    /// The mean absolute difference in Fast Fourier Transform magnitude (in dB).
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

/// Computes a deterministic FNV-1a hash over a series of APU register writes.
///
/// This is used to quickly verify if two different execution runs produced
/// the exact same sequence of audio register manipulations, without needing
/// to store massive arrays of event traces.
///
/// ## Examples
///
/// ```rust
/// use nes_test_harness::{ApuWriteEvent, apu_write_hash};
///
/// let events = vec![
///     ApuWriteEvent { cpu_cycle: 1000, addr: 0x4000, value: 0xFF },
///     ApuWriteEvent { cpu_cycle: 2000, addr: 0x4015, value: 0x01 },
/// ];
///
/// let hash1 = apu_write_hash(&events);
/// let hash2 = apu_write_hash(&events);
/// assert_eq!(hash1, hash2, "The hash is deterministic");
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

/// Computes a deterministic FNV-1a hash over raw PCM audio samples.
///
/// This provides a quick way to verify that an audio stream is bit-for-bit
/// identical to a previous run, acting as a regression test before doing
/// a more expensive waveform comparison.
///
/// ## Examples
///
/// ```rust
/// use nes_test_harness::waveform_hash;
///
/// let audio_chunk = [0i16, 16000, 32000, 16000, 0, -16000, -32000, -16000];
/// let hash = waveform_hash(&audio_chunk);
/// assert_ne!(hash, 0);
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

/// Computes general descriptive statistics for a buffer of raw PCM audio.
///
/// This provides details like peak amplitude, RMS volume, and clipping ratio
/// to help identify issues like completely silent output or overdrive distortion.
///
/// ## Examples
///
/// ```rust
/// use nes_test_harness::audio_stats;
///
/// let silence = [0i16; 1000];
/// let stats = audio_stats(&silence);
/// assert_eq!(stats.sample_count, 1000);
/// assert_eq!(stats.rms, 0.0);
/// assert_eq!(stats.peak, 0);
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

/// Computes a downsampled RMS (Root Mean Square) envelope of an audio buffer.
///
/// This partitions the sample buffer into `window_samples` sized chunks and computes
/// the RMS volume of each chunk, producing a smaller array representing the overall
/// amplitude curve of the audio over time.
///
/// ## Examples
///
/// ```rust
/// use nes_test_harness::rms_envelope;
///
/// let buffer = [1000, -1000, 1000, -1000, 500, -500, 500, -500];
/// // Downsample the 8-sample buffer into an envelope with 4-sample windows.
/// let env = rms_envelope(&buffer, 4);
/// assert_eq!(env.len(), 2);
/// assert!(env[0] > env[1]); // The first window is louder
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

/// Performs a deep comparison of two audio waveforms.
///
/// Rather than an exact bit-for-bit check, this calculates correlation, RMS ratio,
/// and FFT magnitude differences. This allows tests to detect if an emulator's
/// audio output "sounds" the same as a golden standard, even if minor floating-point
/// discrepancies or phase shifts prevent an exact hash match.
///
/// ## Examples
///
/// ```rust
/// use nes_test_harness::compare_waveforms;
///
/// let signal_a = [1000, 2000, 1000, 0];
/// let signal_b = [1000, 2000, 1000, 0];
/// let cmp = compare_waveforms(&signal_a, &signal_b, 4);
/// assert!(cmp.correlation > 0.99);
/// assert!(cmp.fft_mean_abs_db_diff < 1.0);
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

/// Computes the log-magnitude frequency spectrum (in Decibels) of an audio buffer.
///
/// This applies a Hann window and an O(N^2) Discrete Fourier Transform (DFT) to
/// the audio buffer, returning the frequency magnitudes up to the Nyquist frequency
/// (half of the `fft_size`). This is intentionally unoptimized as it's meant for
/// comparing very small audio windows during tests, not for real-time DSP.
///
/// ## Examples
///
/// ```rust
/// use nes_test_harness::fft_log_mag_db;
///
/// let dc_signal = [5000, 5000, 5000, 5000];
/// let spectrum = fft_log_mag_db(&dc_signal, 4);
/// // The returned bins represent frequencies up to the Nyquist limit (size / 2)
/// assert_eq!(spectrum.len(), 2);
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

/// Extracts the iNES or NES 2.0 mapper ID from raw ROM bytes.
///
/// This parses the standard 16-byte header to determine which memory mapping
/// hardware the game expects. Returns `None` if the header is invalid or missing.
///
/// ## Examples
///
/// ```rust
/// use nes_test_harness::{build_homebrew_rom, detect_mapper_id};
///
/// let rom = build_homebrew_rom().unwrap();
/// let mapper = detect_mapper_id(&rom);
/// assert_eq!(mapper, Some(0)); // Homebrew uses NROM (Mapper 0)
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

/// Checks if a given mapper ID is implemented and supported by the `nes-core`.
///
/// This is used in test harnesses (like fuzzing or batch audio tests) to skip
/// ROMs that the emulator cannot physically run yet.
///
/// ## Examples
///
/// ```rust
/// use nes_test_harness::mapper_supported_by_core;
///
/// assert!(mapper_supported_by_core(0)); // NROM is supported
/// assert!(mapper_supported_by_core(4)); // MMC3 is supported
/// assert!(!mapper_supported_by_core(999)); // Unknown mapper
/// ```
#[must_use]
pub fn mapper_supported_by_core(mapper_id: u16) -> bool {
    matches!(
        mapper_id,
        0 | 1 | 2 | 3 | 4 | 7 | 9 | 10 | 11 | 66 | 69 | 71 | 206
    )
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
        for id in [0, 1, 2, 3, 4, 7, 9, 10, 11, 66, 69, 71, 206] {
            assert!(
                mapper_supported_by_core(id),
                "mapper {id} should be supported"
            );
        }
        assert!(!mapper_supported_by_core(5));
        assert!(!mapper_supported_by_core(163));
    }
}
