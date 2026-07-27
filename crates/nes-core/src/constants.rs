//! Global constants defining the core properties of the NES system.
//!
//! This module centralizes critical system-wide values like display dimensions
//! and audio sample rates. Keeping these constants here ensures that all modules
//! (like the PPU and APU) and external crates (like the TUI or Desktop apps)
//! share a single source of truth, preventing "magic number" drift.

/// The width of the NES screen in pixels.
pub const FRAME_WIDTH: usize = 256;

/// The height of the NES screen in pixels.
pub const FRAME_HEIGHT: usize = 240;

/// The number of bytes required to hold a single frame of RGBA pixels.
pub const FRAME_RGBA_BYTES: usize = FRAME_WIDTH * FRAME_HEIGHT * 4;

/// The standard audio sample rate used by the NES core (44.1 kHz).
pub const AUDIO_SAMPLE_RATE: u32 = 44_100;

/// The number of audio samples generated per frame (assuming 60 FPS).
pub const AUDIO_CHUNK_SAMPLES: usize = (AUDIO_SAMPLE_RATE as usize) / 60;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_frame_rgba_bytes_is_exact_buffer_size() {
        // Assert the exact byte size needed for a 256x240 RGBA framebuffer
        assert_eq!(FRAME_RGBA_BYTES, 245_760);
    }

    #[test]
    fn test_audio_chunk_samples_is_correct_for_60hz() {
        // Assert the exact number of samples for 44.1kHz at 60 FPS
        assert_eq!(AUDIO_CHUNK_SAMPLES, 735);
    }
}
