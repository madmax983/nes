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
