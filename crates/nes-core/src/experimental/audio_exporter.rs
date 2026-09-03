//! Experimental tools for exporting raw NES audio into standard media formats.
//!
//! This module provides the `WavExporter` utility, allowing developers or players
//! to easily capture the emulator's audio output and save it as a standard `.wav` file.
//! This is particularly useful for extracting chiptunes, sound effects, or recording
//! the audio of a complete TAS run.

use alloc::vec::Vec;

#[cfg(feature = "nova")]
use crate::constants::AUDIO_SAMPLE_RATE;

/// A utility for recording 16-bit PCM samples and exporting them as a RIFF `.wav` file.
///
/// `WavExporter` acts as an audio sink. You push chunks of samples into it during
/// emulation, and when finished, you can generate a complete `.wav` file in memory
/// ready to be written to disk.
#[cfg(feature = "nova")]
#[derive(Debug, Clone)]
pub struct WavExporter {
    sample_rate: u32,
    samples: Vec<i16>,
}

#[cfg(feature = "nova")]
impl Default for WavExporter {
    fn default() -> Self {
        Self::new(AUDIO_SAMPLE_RATE)
    }
}

#[cfg(feature = "nova")]
impl WavExporter {
    /// Creates a new, empty `WavExporter` with the specified sample rate.
    pub fn new(sample_rate: u32) -> Self {
        Self {
            sample_rate,
            samples: Vec::new(),
        }
    }

    /// Appends a chunk of 16-bit mono PCM samples to the exporter.
    pub fn push_samples(&mut self, new_samples: &[i16]) {
        self.samples.extend_from_slice(new_samples);
    }

    /// Clears all recorded samples, resetting the exporter.
    pub fn clear(&mut self) {
        self.samples.clear();
    }

    /// Generates a complete RIFF `.wav` file from the recorded samples.
    ///
    /// The resulting byte vector includes the standard 44-byte WAV header followed
    /// by the raw PCM data. It can be written directly to a `.wav` file on disk.
    ///
    /// ## Examples
    ///
    /// ```rust
    /// use nes_core::experimental::audio_exporter::WavExporter;
    ///
    /// let mut exporter = WavExporter::new(44100);
    /// exporter.push_samples(&[0, 100, 200, 100, 0, -100, -200, -100]);
    ///
    /// let wav_bytes = exporter.export();
    /// assert_eq!(&wav_bytes[0..4], b"RIFF");
    /// assert_eq!(&wav_bytes[8..12], b"WAVE");
    /// ```
    pub fn export(&self) -> Vec<u8> {
        let num_samples = self.samples.len() as u32;
        let data_size = num_samples * 2; // 2 bytes per i16 sample
        let file_size = 36 + data_size; // 36 bytes for header after the RIFF size

        let mut out = Vec::with_capacity(44 + data_size as usize);

        // RIFF header
        out.extend_from_slice(b"RIFF");
        out.extend_from_slice(&file_size.to_le_bytes());
        out.extend_from_slice(b"WAVE");

        // fmt chunk
        out.extend_from_slice(b"fmt ");
        out.extend_from_slice(&16u32.to_le_bytes()); // Chunk size
        out.extend_from_slice(&1u16.to_le_bytes()); // Audio format (1 = PCM)
        out.extend_from_slice(&1u16.to_le_bytes()); // Num channels (1 = Mono)
        out.extend_from_slice(&self.sample_rate.to_le_bytes()); // Sample rate

        let byte_rate = self.sample_rate * 2; // sample_rate * num_channels * bytes_per_sample
        out.extend_from_slice(&byte_rate.to_le_bytes()); // Byte rate

        let block_align = 2u16; // num_channels * bytes_per_sample
        out.extend_from_slice(&block_align.to_le_bytes()); // Block align

        let bits_per_sample = 16u16;
        out.extend_from_slice(&bits_per_sample.to_le_bytes()); // Bits per sample

        // data chunk
        out.extend_from_slice(b"data");
        out.extend_from_slice(&data_size.to_le_bytes());

        // sample data
        for &sample in &self.samples {
            out.extend_from_slice(&sample.to_le_bytes());
        }

        out
    }
}

#[cfg(all(test, feature = "nova"))]
mod tests {
    use super::*;

    #[test]
    fn can_export_wav_file() {
        let mut exporter = WavExporter::new(44100);
        exporter.push_samples(&[100, -100, 200, -200]);

        let wav = exporter.export();

        // Check header length
        assert_eq!(wav.len(), 44 + 8);

        // Check RIFF magic
        assert_eq!(&wav[0..4], b"RIFF");
        // Check WAVE format
        assert_eq!(&wav[8..12], b"WAVE");
        // Check fmt chunk
        assert_eq!(&wav[12..16], b"fmt ");
        // Check data chunk
        assert_eq!(&wav[36..40], b"data");

        // Verify data size (4 samples * 2 bytes = 8)
        let data_size = u32::from_le_bytes(wav[40..44].try_into().unwrap());
        assert_eq!(data_size, 8);

        // Verify sample values
        let sample0 = i16::from_le_bytes(wav[44..46].try_into().unwrap());
        let sample1 = i16::from_le_bytes(wav[46..48].try_into().unwrap());
        assert_eq!(sample0, 100);
        assert_eq!(sample1, -100);
    }

    #[test]
    fn can_clear_samples() {
        let mut exporter = WavExporter::default();
        exporter.push_samples(&[1, 2, 3]);
        assert_eq!(exporter.samples.len(), 3);
        exporter.clear();
        assert_eq!(exporter.samples.len(), 0);
    }
}
