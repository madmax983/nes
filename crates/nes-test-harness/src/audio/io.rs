use std::fs;
use std::path::Path;

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
