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

#[cfg(test)]
mod tests {
    use super::*;
    use nes_core::NesCore;

    #[test]
    fn test_rewind_speed_lookahead_depth() {
        assert_eq!(RewindSpeed::Single.lookahead_depth(), 1);
        assert_eq!(RewindSpeed::Normal.lookahead_depth(), 16);
        assert_eq!(RewindSpeed::Fast.lookahead_depth(), 32);
        assert_eq!(RewindSpeed::Faster.lookahead_depth(), 64);
    }

    #[test]
    fn test_rewind_speed_frame_skip() {
        assert_eq!(RewindSpeed::Single.frame_skip(), 1);
        assert_eq!(RewindSpeed::Normal.frame_skip(), 1);
        assert_eq!(RewindSpeed::Fast.frame_skip(), 2);
        assert_eq!(RewindSpeed::Faster.frame_skip(), 4);
    }

    #[test]
    fn test_rewind_cursor_pop_frame() {
        let core = NesCore::new();
        let snapshot = core.save_state();
        let mut cursor = RewindCursor::new(100, RewindSpeed::Normal);

        cursor.lookahead.push_back((99, snapshot.clone()));

        let popped = cursor.pop_frame().unwrap();
        assert_eq!(popped.0, 99);
        assert_eq!(cursor.current_frame, 99);

        // Empty pop
        assert!(cursor.pop_frame().is_none());
    }
}
