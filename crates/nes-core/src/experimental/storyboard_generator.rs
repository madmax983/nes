#![cfg(feature = "nova")]

#[cfg(feature = "tas")]
use crate::Command;
use crate::CoreError;
use crate::NesCore;
use crate::bmp::encode_bmp;
use crate::constants::{FRAME_HEIGHT, FRAME_WIDTH};
#[cfg(feature = "tas")]
use crate::tas::TasMovie;

/// Generates visual storyboards (stitched thumbnail grids) from emulator execution.
pub struct StoryboardGenerator {
    thumbnails: Vec<Vec<u8>>,
    thumbnail_width: usize,
    thumbnail_height: usize,
}

impl StoryboardGenerator {
    /// Creates a new generator targeting the specified thumbnail dimensions.
    #[must_use]
    pub fn new(thumbnail_width: usize, thumbnail_height: usize) -> Self {
        let thumbnail_width = thumbnail_width.max(1);
        let thumbnail_height = thumbnail_height.max(1);
        Self {
            thumbnails: Vec::new(),
            thumbnail_width,
            thumbnail_height,
        }
    }

    /// Captures the current emulator frame, downscales it, and stores the thumbnail.
    pub fn capture_frame(&mut self, core: &NesCore) {
        let full_frame = core.framebuffer_rgba();
        let mut thumbnail = vec![0_u8; self.thumbnail_width * self.thumbnail_height * 4];

        for y in 0..self.thumbnail_height {
            for x in 0..self.thumbnail_width {
                // Nearest neighbor downscaling
                let src_x = (x * FRAME_WIDTH) / self.thumbnail_width;
                let src_y = (y * FRAME_HEIGHT) / self.thumbnail_height;

                let src_idx = (src_y * FRAME_WIDTH + src_x) * 4;
                let dest_idx = (y * self.thumbnail_width + x) * 4;

                thumbnail[dest_idx] = full_frame[src_idx];
                thumbnail[dest_idx + 1] = full_frame[src_idx + 1];
                thumbnail[dest_idx + 2] = full_frame[src_idx + 2];
                thumbnail[dest_idx + 3] = full_frame[src_idx + 3];
            }
        }

        self.thumbnails.push(thumbnail);
    }

    /// Automatically steps through a TAS movie and captures frames at a set interval.
    #[cfg(feature = "tas")]
    pub fn generate_tas_storyboard(
        core: &mut NesCore,
        movie: &TasMovie,
        capture_interval_frames: u32,
        thumbnail_width: usize,
        thumbnail_height: usize,
    ) -> Result<Self, CoreError> {
        let mut generator = Self::new(thumbnail_width, thumbnail_height);
        let mut elapsed_frames = 0;

        for run in movie.runs() {
            if run.frames == 0 {
                continue;
            }
            core.execute(Command::SetControllerState(run.controller1_bits))?;
            core.execute(Command::SetController2State(run.controller2_bits))?;

            for _ in 0..run.frames {
                if elapsed_frames % capture_interval_frames == 0 {
                    generator.capture_frame(core);
                }
                core.execute(Command::StepFrame)?;
                elapsed_frames += 1;
            }
        }

        // Capture the final frame if it wasn't just captured
        if elapsed_frames > 0 {
            generator.capture_frame(core);
        }

        Ok(generator)
    }

    /// Stitches all captured thumbnails into a grid with the specified number of columns and returns a BMP.
    pub fn export_bmp(&self, columns: usize) -> Result<Vec<u8>, String> {
        if self.thumbnails.is_empty() {
            return Err("No frames captured".to_string());
        }

        let cols = columns.max(1);
        let rows = self.thumbnails.len().div_ceil(cols);

        let total_width = cols * self.thumbnail_width;
        let total_height = rows * self.thumbnail_height;

        let mut rgba = vec![0_u8; total_width * total_height * 4];

        for (i, thumbnail) in self.thumbnails.iter().enumerate() {
            let col = i % cols;
            let row = i / cols;

            let base_x = col * self.thumbnail_width;
            let base_y = row * self.thumbnail_height;

            for y in 0..self.thumbnail_height {
                for x in 0..self.thumbnail_width {
                    let src_idx = (y * self.thumbnail_width + x) * 4;
                    let dest_idx = ((base_y + y) * total_width + (base_x + x)) * 4;

                    rgba[dest_idx] = thumbnail[src_idx];
                    rgba[dest_idx + 1] = thumbnail[src_idx + 1];
                    rgba[dest_idx + 2] = thumbnail[src_idx + 2];
                    rgba[dest_idx + 3] = thumbnail[src_idx + 3];
                }
            }
        }

        encode_bmp(total_width, total_height, &rgba)
    }
}

#[cfg(all(test, feature = "nova", feature = "tas"))]
mod tests {
    use super::*;
    use crate::tas::TasFrameRun;

    #[test]
    fn storyboard_generator_creates_valid_bmp() {
        let mut core = NesCore::new();
        let runs = vec![TasFrameRun::new(0, 0, 10)];
        let movie = TasMovie::from_runs(runs);

        let generator =
            StoryboardGenerator::generate_tas_storyboard(&mut core, &movie, 5, 64, 60).unwrap();
        // 10 frames total, interval 5, so captures at frame 0, 5, and the final frame (10 -> but wait 10 loops so it steps 10 times, capturing at 0, 5, and since 10 % 5 == 0, the final frame is handled)
        let bmp_data = generator.export_bmp(2).unwrap();

        assert_eq!(&bmp_data[0..2], b"BM");
    }
}
