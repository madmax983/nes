//! Rewind cursor: tracks position and speed during active rewind.

use std::collections::VecDeque;

use nes_core::CoreSnapshot;

/// Playback speed during rewind.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RewindSpeed {
    /// Single frame step (tap rewind).
    Single,
    /// Normal hold speed.
    Normal,
    /// Fast (hold longer: 2× frame skip).
    Fast,
    /// Turbo (hold longest: 4× frame skip).
    Faster,
}

impl RewindSpeed {
    /// How many frames ahead to speculatively pre-fetch.
    pub fn lookahead_depth(self) -> usize {
        match self {
            Self::Single => 1,
            Self::Normal => 16,
            Self::Fast => 32,
            Self::Faster => 64,
        }
    }

    /// How many timeline frames to skip per displayed frame.
    pub fn frame_skip(self) -> usize {
        match self {
            Self::Single | Self::Normal => 1,
            Self::Fast => 2,
            Self::Faster => 4,
        }
    }
}

/// Tracks the current rewind position and a pre-fetched lookahead buffer.
#[derive(Debug)]
pub struct RewindCursor {
    /// Current logical frame id the cursor is sitting at.
    pub current_frame: u64,
    /// Active rewind speed.
    pub speed: RewindSpeed,
    /// Pre-fetched (frame_id, snapshot) pairs buffered for smooth playback.
    pub lookahead: VecDeque<(u64, CoreSnapshot)>,
}

impl RewindCursor {
    /// Create a cursor starting at `current_frame`.
    pub fn new(current_frame: u64, speed: RewindSpeed) -> Self {
        Self {
            current_frame,
            speed,
            lookahead: VecDeque::new(),
        }
    }

    /// Pop the next pre-fetched frame. Updates `current_frame` on success.
    pub fn pop_frame(&mut self) -> Option<(u64, CoreSnapshot)> {
        let frame = self.lookahead.pop_front()?;
        self.current_frame = frame.0;
        Some(frame)
    }
}
