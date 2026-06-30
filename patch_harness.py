import re

with open('crates/nes-test-harness/src/lib.rs', 'r') as f:
    content = f.read()

# Add docs for ApuWriteEvent
content = content.replace(
    'pub struct ApuWriteEvent {',
    '/// A record of a CPU write to an APU register.\npub struct ApuWriteEvent {'
)
content = re.sub(
    r'(\s+)pub cpu_cycle: u64,',
    r'\1/// The exact CPU cycle when the write occurred.\1pub cpu_cycle: u64,',
    content
)
content = re.sub(
    r'(\s+)pub addr: u16,',
    r'\1/// The destination address of the APU register.\1pub addr: u16,',
    content
)
content = re.sub(
    r'(\s+)pub value: u8,',
    r'\1/// The byte value written.\1pub value: u8,',
    content
)

# Add docs for AudioStats
content = content.replace(
    'pub struct AudioStats {',
    '/// Statistical analysis of a PCM audio buffer.\npub struct AudioStats {'
)
content = re.sub(
    r'(\s+)pub sample_count: usize,',
    r'\1/// Number of samples analyzed.\1pub sample_count: usize,',
    content
)
content = re.sub(
    r'(\s+)pub rms: f64,',
    r'\1/// Root Mean Square value (perceived loudness).\1pub rms: f64,',
    content
)
content = re.sub(
    r'(\s+)pub peak: i16,',
    r'\1/// Absolute maximum sample value.\1pub peak: i16,',
    content
)
content = re.sub(
    r'(\s+)pub dc_offset: f64,',
    r'\1/// Mean value indicating DC bias.\1pub dc_offset: f64,',
    content
)
content = re.sub(
    r'(\s+)pub clipping_ratio: f64,',
    r'\1/// Percentage of samples hitting the maximum/minimum range bounds.\1pub clipping_ratio: f64,',
    content
)

# Add docs for WaveformComparison
content = content.replace(
    'pub struct WaveformComparison {',
    '/// Results of comparing two audio waveforms.\npub struct WaveformComparison {'
)
content = re.sub(
    r'(\s+)pub samples_compared: usize,',
    r'\1/// Number of samples compared.\1pub samples_compared: usize,',
    content
)
content = re.sub(
    r'(\s+)pub correlation: f64,',
    r'\1/// Cross-correlation factor between the waveforms (-1.0 to 1.0).\1pub correlation: f64,',
    content
)
content = re.sub(
    r'(\s+)pub rms_ratio: f64,',
    r'\1/// Ratio of RMS values between the two waveforms.\1pub rms_ratio: f64,',
    content
)
content = re.sub(
    r'(\s+)pub fft_mean_abs_db_diff: f64,',
    r'\1/// Mean absolute difference in magnitude (dB) between FFT frequency bins.\1pub fft_mean_abs_db_diff: f64,',
    content
)

# Add docs for functions
content = content.replace(
    'pub fn apu_write_hash(writes: &[ApuWriteEvent]) -> u64 {',
    '/// Generates a simplistic hash from a sequence of APU write events.\npub fn apu_write_hash(writes: &[ApuWriteEvent]) -> u64 {'
)
content = content.replace(
    'pub fn waveform_hash(samples: &[i16]) -> u64 {',
    '/// Generates a simplistic hash from a sequence of audio samples.\npub fn waveform_hash(samples: &[i16]) -> u64 {'
)
content = content.replace(
    'pub fn audio_stats(samples: &[i16]) -> AudioStats {',
    '/// Computes statistical metrics on a raw PCM buffer.\npub fn audio_stats(samples: &[i16]) -> AudioStats {'
)
content = content.replace(
    'pub fn rms_envelope(samples: &[i16], window_samples: usize) -> Vec<f64> {',
    '/// Extracts the RMS envelope of an audio buffer over a sliding window.\npub fn rms_envelope(samples: &[i16], window_samples: usize) -> Vec<f64> {'
)
content = content.replace(
    'pub fn compare_waveforms(lhs: &[i16], rhs: &[i16], fft_size: usize) -> WaveformComparison {',
    '/// Performs an in-depth comparison of two audio waveforms.\npub fn compare_waveforms(lhs: &[i16], rhs: &[i16], fft_size: usize) -> WaveformComparison {'
)
content = content.replace(
    'pub fn fft_log_mag_db(samples: &[i16], fft_size: usize) -> Vec<f64> {',
    '/// Calculates the frequency magnitude spectrum of an audio buffer.\npub fn fft_log_mag_db(samples: &[i16], fft_size: usize) -> Vec<f64> {'
)
content = content.replace(
    'pub fn detect_mapper_id(rom_bytes: &[u8]) -> Option<u16> {',
    '/// Attempts to parse the iNES mapper ID from raw ROM bytes.\npub fn detect_mapper_id(rom_bytes: &[u8]) -> Option<u16> {'
)
content = content.replace(
    'pub fn mapper_supported_by_core(mapper_id: u16) -> bool {',
    '/// Checks if the core emulator supports a given mapper ID.\npub fn mapper_supported_by_core(mapper_id: u16) -> bool {'
)

with open('crates/nes-test-harness/src/lib.rs', 'w') as f:
    f.write(content)


with open('crates/nes-test-harness/src/homebrew.rs', 'r') as f:
    content = f.read()

content = content.replace(
    'pub fn default_homebrew_rom_path() -> PathBuf {',
    '/// Path to the auto-built homebrew ROM.\npub fn default_homebrew_rom_path() -> PathBuf {'
)
with open('crates/nes-test-harness/src/homebrew.rs', 'w') as f:
    f.write(content)

with open('crates/nes-test-harness/src/rom_paths.rs', 'r') as f:
    content = f.read()
content = content.replace(
    'pub fn smb_rom_path() -> String {',
    '/// Path to the SMB ROM specified in the config.\npub fn smb_rom_path() -> String {'
)
content = content.replace(
    'pub fn nestest_rom_path() -> String {',
    '/// Path to the nestest ROM specified in the config.\npub fn nestest_rom_path() -> String {'
)
content = content.replace(
    'pub fn blargg_cpu_rom_path() -> String {',
    '/// Path to the blargg CPU test ROM specified in the config.\npub fn blargg_cpu_rom_path() -> String {'
)
content = content.replace(
    'pub fn bbbradsmith_audio_suite_rom_paths() -> Vec<String> {',
    '/// Paths to the Brad Smith audio suite ROMs specified in the config.\npub fn bbbradsmith_audio_suite_rom_paths() -> Vec<String> {'
)
content = content.replace(
    'pub fn bbbradsmith_audio_golden_dir_path() -> String {',
    '/// Path to the golden directory for audio tests.\npub fn bbbradsmith_audio_golden_dir_path() -> String {'
)

with open('crates/nes-test-harness/src/rom_paths.rs', 'w') as f:
    f.write(content)
