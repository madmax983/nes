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
    fn constants_are_correct() {
        assert_eq!(FRAME_WIDTH, 256);
        assert_eq!(FRAME_HEIGHT, 240);
        assert_eq!(FRAME_RGBA_BYTES, 256 * 240 * 4);
        assert_eq!(AUDIO_SAMPLE_RATE, 44_100);
        assert_eq!(AUDIO_CHUNK_SAMPLES, 44_100 / 60);
    }
}
