use std::collections::VecDeque;

use nes_core::{FRAME_HEIGHT, FRAME_WIDTH};

/// Converts the NES RGBA framebuffer into a grayscale image at the requested size.
///
/// # Panics
///
/// Panics if `width` or `height` is zero, or if `rgba` does not match the
/// expected NES framebuffer size.
#[must_use]
pub fn downsample_grayscale(rgba: &[u8], width: usize, height: usize) -> Vec<f32> {
    assert!(width > 0, "downsample width must be greater than zero");
    assert!(height > 0, "downsample height must be greater than zero");

    let expected_len = FRAME_WIDTH * FRAME_HEIGHT * 4;
    assert_eq!(
        rgba.len(),
        expected_len,
        "rgba buffer must match the NES framebuffer size"
    );

    let mut out = vec![0.0; width * height];
    for y in 0..height {
        for x in 0..width {
            let src_x = x * FRAME_WIDTH / width;
            let src_y = y * FRAME_HEIGHT / height;
            let idx = (src_y * FRAME_WIDTH + src_x) * 4;
            let r = f32::from(rgba[idx]);
            let g = f32::from(rgba[idx + 1]);
            let b = f32::from(rgba[idx + 2]);
            out[y * width + x] = (0.299 * r + 0.587 * g + 0.114 * b) / 255.0;
        }
    }
    out
}

#[derive(Debug, Clone)]
pub struct FrameStack {
    max_frames: usize,
    frame_len: usize,
    frames: VecDeque<Vec<f32>>,
}

impl FrameStack {
    /// Creates a bounded frame stack with a fixed per-frame element count.
    ///
    /// # Panics
    ///
    /// Panics if `max_frames` or `frame_len` is zero.
    #[must_use]
    pub fn new(max_frames: usize, frame_len: usize) -> Self {
        assert!(max_frames > 0, "frame stack must keep at least one frame");
        assert!(frame_len > 0, "frame length must be greater than zero");
        Self {
            max_frames,
            frame_len,
            frames: VecDeque::new(),
        }
    }

    /// Pushes one preprocessed frame into the stack, evicting the oldest frame if full.
    ///
    /// # Panics
    ///
    /// Panics if `frame.len()` does not match the configured frame length.
    pub fn push(&mut self, frame: Vec<f32>) {
        assert_eq!(frame.len(), self.frame_len);
        if self.frames.len() == self.max_frames {
            self.frames.pop_front();
        }
        self.frames.push_back(frame);
    }

    #[must_use]
    pub fn as_slices(&self) -> Vec<&[f32]> {
        self.frames.iter().map(Vec::as_slice).collect()
    }
}
