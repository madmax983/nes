import re

with open('crates/nes-test-harness/src/lib.rs', 'r') as f:
    content = f.read()

content = content.replace(
    '#[derive(Debug, Clone, Copy, PartialEq, Eq)]\npub struct ApuWriteEvent {',
    '/// Represents a single write operation to an APU register during execution.\n#[derive(Debug, Clone, Copy, PartialEq, Eq)]\npub struct ApuWriteEvent {'
)
content = content.replace(
    '    pub cpu_cycle: u64,\n    pub addr: u16,\n    pub value: u8,',
    '''    /// The exact cycle count at which the write occurred.
    pub cpu_cycle: u64,
    /// The memory address written to (e.g., $4000).
    pub addr: u16,
    /// The 8-bit value written.
    pub value: u8,'''
)

content = content.replace(
    '#[derive(Debug, Clone, Copy, PartialEq)]\npub struct AudioStats {',
    '/// Statistical metrics extracted from a captured audio buffer.\n#[derive(Debug, Clone, Copy, PartialEq)]\npub struct AudioStats {'
)
content = content.replace(
    '    pub sample_count: usize,\n    pub rms: f64,\n    pub peak: i16,\n    pub dc_offset: f64,\n    pub clipping_ratio: f64,',
    '''    /// The total number of audio samples.
    pub sample_count: usize,
    /// Root Mean Square (RMS) volume of the audio.
    pub rms: f64,
    /// Maximum absolute amplitude reached.
    pub peak: i16,
    /// The average displacement from zero.
    pub dc_offset: f64,
    /// The fraction of samples reaching maximum amplitude bounds.
    pub clipping_ratio: f64,'''
)

content = content.replace(
    '#[derive(Debug, Clone, PartialEq)]\npub struct WaveformComparison {',
    '/// Metrics comparing an emulator-generated waveform against a golden output.\n#[derive(Debug, Clone, PartialEq)]\npub struct WaveformComparison {'
)
content = content.replace(
    '    pub samples_compared: usize,\n    pub correlation: f64,\n    pub rms_ratio: f64,\n    pub fft_mean_abs_db_diff: f64,',
    '''    /// The length of the comparison window.
    pub samples_compared: usize,
    /// The Pearson correlation coefficient [-1.0, 1.0].
    pub correlation: f64,
    /// The ratio of RMS volume between the streams.
    pub rms_ratio: f64,
    /// Mean absolute difference across frequency bins in the FFT spectrum.
    pub fft_mean_abs_db_diff: f64,'''
)

content = content.replace(
    '#[must_use]\npub fn apu_write_hash(writes: &[ApuWriteEvent]) -> u64 {',
    '/// Computes a fast deterministic hash of an APU write sequence.\n///\n/// Uses FNV-1a to turn the sequence into a single 64-bit fingerprint for test assertions.\n#[must_use]\npub fn apu_write_hash(writes: &[ApuWriteEvent]) -> u64 {'
)

content = content.replace(
    '#[must_use]\npub fn waveform_hash(samples: &[i16]) -> u64 {',
    '/// Computes a fast deterministic hash of an audio waveform.\n///\n/// Uses FNV-1a to turn the sequence into a single 64-bit fingerprint for test assertions.\n#[must_use]\npub fn waveform_hash(samples: &[i16]) -> u64 {'
)

content = content.replace(
    '#[must_use]\npub fn audio_stats(samples: &[i16]) -> AudioStats {',
    '/// Calculates basic audio statistics (RMS, peak, clipping) for an audio stream.\n#[must_use]\npub fn audio_stats(samples: &[i16]) -> AudioStats {'
)

content = content.replace(
    '#[must_use]\npub fn rms_envelope(samples: &[i16], window_samples: usize) -> Vec<f64> {',
    '/// Extracts the RMS volume envelope from an audio stream.\n///\n/// Divides the stream into chunks of `window_samples` and computes the RMS volume of each chunk.\n#[must_use]\npub fn rms_envelope(samples: &[i16], window_samples: usize) -> Vec<f64> {'
)

content = content.replace(
    '#[must_use]\npub fn compare_waveforms(lhs: &[i16], rhs: &[i16], fft_size: usize) -> WaveformComparison {',
    '/// Compares two audio waveforms returning metrics for correlation and frequency drift.\n#[must_use]\npub fn compare_waveforms(lhs: &[i16], rhs: &[i16], fft_size: usize) -> WaveformComparison {'
)

content = content.replace(
    '#[must_use]\npub fn fft_log_mag_db(samples: &[i16], fft_size: usize) -> Vec<f64> {',
    '/// Computes a basic discrete fourier transform of an audio segment returning magnitudes in decibels.\n#[must_use]\npub fn fft_log_mag_db(samples: &[i16], fft_size: usize) -> Vec<f64> {'
)

content = content.replace(
    '#[must_use]\npub fn detect_mapper_id(rom_bytes: &[u8]) -> Option<u16> {',
    '/// Parses the iNES header of a ROM file to determine its memory mapper ID.\n/// Returns `None` if the header is missing or malformed.\n#[must_use]\npub fn detect_mapper_id(rom_bytes: &[u8]) -> Option<u16> {'
)

with open('crates/nes-test-harness/src/lib.rs', 'w') as f:
    f.write(content)
