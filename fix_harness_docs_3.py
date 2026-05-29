def insert_docs(file_path):
    with open(file_path, 'r') as f:
        content = f.read().split('\n')

    for i in range(len(content) - 1, -1, -1):
        line = content[i]

        doc = ""
        if "pub struct ApuWriteEvent" in line:
            doc = "/// Represents a single write operation to an APU register."
        elif "pub cpu_cycle: u64" in line:
            doc = "    /// The absolute CPU cycle timestamp when the write occurred."
        elif "pub addr: u16" in line and "ApuWriteEvent" in '\n'.join(content[max(0, i-5):i]):
            doc = "    /// The 16-bit memory address of the APU register."
        elif "pub value: u8" in line and "ApuWriteEvent" in '\n'.join(content[max(0, i-6):i]):
            doc = "    /// The 8-bit value written to the register."
        elif "pub struct AudioStats" in line:
            doc = "/// Statistical characteristics of an audio waveform."
        elif "pub sample_count: usize" in line and "AudioStats" in '\n'.join(content[max(0, i-5):i]):
            doc = "    /// Total number of audio samples."
        elif "pub rms: f64" in line and "AudioStats" in '\n'.join(content[max(0, i-6):i]):
            doc = "    /// Root Mean Square energy of the waveform."
        elif "pub peak: i16" in line and "AudioStats" in '\n'.join(content[max(0, i-7):i]):
            doc = "    /// Maximum absolute amplitude reached."
        elif "pub dc_offset: f64" in line:
            doc = "    /// Average amplitude offset from zero."
        elif "pub clipping_ratio: f64" in line:
            doc = "    /// Ratio of samples that hit maximum/minimum bounds."
        elif "pub struct WaveformComparison" in line:
            doc = "/// Metrics comparing two audio waveforms for similarity."
        elif "pub samples_compared: usize" in line and "WaveformComparison" in '\n'.join(content[max(0, i-5):i]):
            doc = "    /// Number of overlapping samples compared."
        elif "pub correlation: f64" in line and "WaveformComparison" in '\n'.join(content[max(0, i-6):i]):
            doc = "    /// Pearson correlation coefficient between the waveforms."
        elif "pub rms_ratio: f64" in line and "WaveformComparison" in '\n'.join(content[max(0, i-7):i]):
            doc = "    /// Ratio of RMS energies between the two waveforms."
        elif "pub fft_mean_abs_db_diff: f64" in line:
            doc = "    /// Mean absolute difference of FFT magnitudes in decibels."
        elif "pub fn apu_write_hash(" in line:
            doc = "/// Computes a deterministic hash over a sequence of APU write events.\n///\n/// ## Examples\n/// ```\n/// use nes_test_harness::{apu_write_hash, ApuWriteEvent};\n/// let hash = apu_write_hash(&[]);\n/// assert_eq!(hash, 14695981039346656037);\n/// ```"
        elif "pub fn audio_stats(" in line:
            doc = "/// Calculates statistical metrics for a buffer of audio samples.\n///\n/// ## Examples\n/// ```\n/// use nes_test_harness::audio_stats;\n/// let stats = audio_stats(&[0, 16000, -16000]);\n/// assert_eq!(stats.sample_count, 3);\n/// ```"
        elif "pub fn compare_waveforms(" in line:
            doc = "/// Compares two audio waveforms and returns similarity metrics.\n///\n/// ## Examples\n/// ```\n/// use nes_test_harness::compare_waveforms;\n/// let a = [0, 1000, 0, -1000];\n/// let b = [0, 900, 0, -900];\n/// let comp = compare_waveforms(&a, &b, 4);\n/// assert!(comp.correlation > 0.99);\n/// ```"
        elif "pub fn extract_ines_chr_rom(" in line:
            doc = "/// Extracts the CHR ROM section from an iNES formatted ROM.\n///\n/// ## Examples\n/// ```no_run\n/// use nes_test_harness::extract_ines_chr_rom;\n/// let chr = extract_ines_chr_rom(b\"NES\\x1A...\").unwrap();\n/// ```"
        elif "pub fn verify_ppu_pattern_table(" in line:
            doc = "/// Verifies the PPU's pattern table against a known CHR ROM.\n///\n/// ## Examples\n/// ```no_run\n/// use nes_core::NesCore;\n/// use nes_test_harness::verify_ppu_pattern_table;\n/// let mut core = NesCore::new();\n/// verify_ppu_pattern_table(&core, b\"CHR\");\n/// ```"
        elif "pub fn create_test_wav_from_pcm(" in line:
            doc = "/// Creates a WAV file containing test audio data.\n///\n/// ## Examples\n/// ```no_run\n/// use nes_test_harness::create_test_wav_from_pcm;\n/// create_test_wav_from_pcm(\"test.wav\", &[0, 1000, 2000]);\n/// ```"
        elif "pub fn normalize_audio(" in line:
            doc = "/// Normalizes an audio buffer to a specified peak level.\n///\n/// ## Examples\n/// ```\n/// use nes_test_harness::normalize_audio;\n/// let mut audio = vec![0, 16000, -16000];\n/// normalize_audio(&mut audio, 32000.0);\n/// assert_eq!(audio[1], 32000);\n/// ```"
        elif "pub fn apply_lowpass(" in line:
            doc = "/// Applies a lowpass filter to an audio buffer.\n///\n/// ## Examples\n/// ```\n/// use nes_test_harness::apply_lowpass;\n/// let mut audio = vec![0, 32000, -32000];\n/// apply_lowpass(&mut audio, 44100.0, 1000.0);\n/// ```"
        elif "pub fn waveform_hash(" in line:
            doc = "/// Computes a deterministic hash for a buffer of audio samples.\n///\n/// ## Examples\n/// ```\n/// use nes_test_harness::waveform_hash;\n/// let hash = waveform_hash(&[0, 100, -100]);\n/// assert_eq!(hash, 3873019965383505159);\n/// ```"
        elif "pub fn rms_envelope(" in line:
            doc = "/// Calculates the RMS envelope of an audio waveform over a sliding window.\n///\n/// ## Examples\n/// ```\n/// use nes_test_harness::rms_envelope;\n/// let env = rms_envelope(&[0, 16000, -16000], 2);\n/// assert_eq!(env.len(), 2);\n/// ```"
        elif "pub fn compute_psnr(" in line:
            doc = "/// Computes PSNR for images.\n///\n/// ## Examples\n/// ```\n/// use nes_test_harness::compute_psnr;\n/// compute_psnr(&[0], &[0]);\n/// ```"
        elif "pub fn compute_ssim(" in line:
            doc = "/// Computes SSIM for images.\n///\n/// ## Examples\n/// ```\n/// use nes_test_harness::compute_ssim;\n/// compute_ssim(&[0], &[0]);\n/// ```"
        elif "pub fn fft_log_mag_db(" in line:
            doc = "/// Computes the FFT magnitude in decibels for a set of audio samples.\n///\n/// ## Examples\n/// ```\n/// use nes_test_harness::fft_log_mag_db;\n/// let db = fft_log_mag_db(&[0, 16000, -16000, 0], 4);\n/// assert_eq!(db.len(), 2);\n/// ```"
        elif "pub fn detect_mapper_id(" in line:
            doc = "/// Detects the mapper ID from an iNES ROM header.\n///\n/// ## Examples\n/// ```\n/// use nes_test_harness::detect_mapper_id;\n/// let mut header = vec![0x4E, 0x45, 0x53, 0x1A, 2, 1, 0x00, 0x00, 0, 0, 0, 0, 0, 0, 0, 0];\n/// assert_eq!(detect_mapper_id(&header), Some(0));\n/// header[6] = 0x40;\n/// assert_eq!(detect_mapper_id(&header), Some(4));\n/// ```"
        elif "pub fn mapper_supported_by_core(" in line:
            doc = "/// Checks if a given mapper ID is supported by the `nes-core` crate.\n///\n/// ## Examples\n/// ```\n/// use nes_test_harness::mapper_supported_by_core;\n/// assert!(mapper_supported_by_core(0));\n/// assert!(!mapper_supported_by_core(999));\n/// ```"

        if doc:
            # Check if there are attributes above it
            insert_idx = i
            while insert_idx > 0 and content[insert_idx - 1].strip().startswith('#['):
                insert_idx -= 1
            if insert_idx > 0 and content[insert_idx - 1].strip().startswith('///'):
                continue
            content.insert(insert_idx, doc)

    with open(file_path, 'w') as f:
        f.write('\n'.join(content))

insert_docs("crates/nes-test-harness/src/lib.rs")

def insert_rom_docs(file_path):
    with open(file_path, 'r') as f:
        content = f.read().split('\n')

    for i in range(len(content) - 1, -1, -1):
        line = content[i]
        doc = ""
        if "pub fn smb_rom_path(" in line:
            doc = "/// Returns the configured path for the Super Mario Bros ROM.\n///\n/// ## Examples\n/// ```no_run\n/// use nes_test_harness::rom_paths::smb_rom_path;\n/// let path = smb_rom_path();\n/// ```"
        elif "pub fn nestest_rom_path(" in line:
            doc = "/// Returns the configured path for the nestest ROM.\n///\n/// ## Examples\n/// ```no_run\n/// use nes_test_harness::rom_paths::nestest_rom_path;\n/// let path = nestest_rom_path();\n/// ```"
        elif "pub fn blargg_cpu_rom_path(" in line:
            doc = "/// Returns the configured path for Blargg's CPU tests.\n///\n/// ## Examples\n/// ```no_run\n/// use nes_test_harness::rom_paths::blargg_cpu_rom_path;\n/// let path = blargg_cpu_rom_path();\n/// ```"
        elif "pub fn bbbradsmith_audio_suite_rom_paths(" in line:
            doc = "/// Returns paths to all ROMs in the bbbradsmith audio test suite.\n///\n/// ## Examples\n/// ```no_run\n/// use nes_test_harness::rom_paths::bbbradsmith_audio_suite_rom_paths;\n/// let paths = bbbradsmith_audio_suite_rom_paths();\n/// ```"
        elif "pub fn bbbradsmith_audio_golden_dir_path(" in line:
            doc = "/// Returns the path to golden audio reference files.\n///\n/// ## Examples\n/// ```no_run\n/// use nes_test_harness::rom_paths::bbbradsmith_audio_golden_dir_path;\n/// let dir = bbbradsmith_audio_golden_dir_path();\n/// ```"

        if doc:
            insert_idx = i
            while insert_idx > 0 and content[insert_idx - 1].strip().startswith('#['):
                insert_idx -= 1
            if insert_idx > 0 and content[insert_idx - 1].strip().startswith('///'):
                continue
            content.insert(insert_idx, doc)

    with open(file_path, 'w') as f:
        f.write('\n'.join(content))

insert_rom_docs("crates/nes-test-harness/src/rom_paths.rs")

def insert_homebrew_docs(file_path):
    with open(file_path, 'r') as f:
        content = f.read().split('\n')

    for i in range(len(content) - 1, -1, -1):
        line = content[i]
        doc = ""
        if "pub fn default_homebrew_rom_path(" in line:
            doc = "/// Returns the default file path for the generated homebrew ROM.\n///\n/// ## Examples\n/// ```\n/// use nes_test_harness::default_homebrew_rom_path;\n/// let path = default_homebrew_rom_path();\n/// assert!(path.ends_with(\"homebrew.nes\"));\n/// ```"

        if doc:
            insert_idx = i
            while insert_idx > 0 and content[insert_idx - 1].strip().startswith('#['):
                insert_idx -= 1
            if insert_idx > 0 and content[insert_idx - 1].strip().startswith('///'):
                continue
            content.insert(insert_idx, doc)

    with open(file_path, 'w') as f:
        f.write('\n'.join(content))

insert_homebrew_docs("crates/nes-test-harness/src/homebrew.rs")
