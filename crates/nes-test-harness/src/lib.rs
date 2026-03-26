use std::f64::consts::PI;
use std::fs;
use std::path::Path;

mod homebrew;
pub mod rom_paths;

pub use homebrew::{build_homebrew_rom, default_homebrew_rom_path, write_homebrew_rom};
pub use rom_paths::*;

use nes_core::{Command, CoreError, NesCore, cpu::CpuBusAccessKind};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ApuWriteEvent {
    pub cpu_cycle: u64,
    pub addr: u16,
    pub value: u8,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AudioStats {
    pub sample_count: usize,
    pub rms: f64,
    pub peak: i16,
    pub dc_offset: f64,
    pub clipping_ratio: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct WaveformComparison {
    pub samples_compared: usize,
    pub correlation: f64,
    pub rms_ratio: f64,
    pub fft_mean_abs_db_diff: f64,
}

const INES_HEADER_LEN: usize = 16;
const INES_MAGIC: [u8; 4] = [0x4E, 0x45, 0x53, 0x1A];

pub fn collect_apu_register_writes(
    core: &mut NesCore,
    cpu_steps: u32,
) -> Result<Vec<ApuWriteEvent>, CoreError> {
    let mut writes = Vec::new();
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

pub fn collect_audio_for_frames(core: &mut NesCore, frames: u32) -> Result<Vec<i16>, CoreError> {
    let mut samples = Vec::new();
    for _ in 0..frames {
        core.execute(Command::StepFrame)?;
        samples.extend(core.audio_chunk_i16());
    }
    Ok(samples)
}

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

#[must_use]
pub fn waveform_hash(samples: &[i16]) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325_u64;
    for sample in samples {
        hash ^= u64::from(*sample as u16);
        hash = hash.wrapping_mul(0x0000_0001_0000_01b3);
    }
    hash
}

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

pub fn write_pcm_i16le(path: &Path, samples: &[i16]) -> Result<(), String> {
    let mut bytes = Vec::with_capacity(samples.len() * 2);
    for sample in samples {
        bytes.extend_from_slice(&sample.to_le_bytes());
    }
    fs::write(path, bytes).map_err(|err| format!("failed to write '{}': {err}", path.display()))
}

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
        assert_ne!(apu_write_hash(&writes), 0);
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
