//! Experimental spatial themer that selectively applies visual filters.
//!
//! By combining `OamSpatialQuery` with `ThemeFilter`, this module allows selectively
//! applying themes (like Sepia or Gameboy) to only specific regions of the screen
//! that contain tracked sprites.

#[cfg(feature = "nova")]
use crate::experimental::oam_spatial_query::OamSpatialQuery;
#[cfg(feature = "nova")]
use crate::experimental::theme_filter::{Theme, ThemeFilter};

#[cfg(feature = "nova")]
/// Selectively applies themes to specific on-screen entities.
pub struct SpatialThemer;

#[cfg(feature = "nova")]
impl SpatialThemer {
    /// Applies a specific `Theme` to bounding boxes around all sprites found in OAM.
    ///
    /// This demonstrates combining spatial queries with post-processing filters,
    /// enabling effects like "highlighting enemies in red" or "rendering the player
    /// in Gameboy colors".
    pub fn apply_to_sprites(
        query: &OamSpatialQuery,
        framebuffer: &mut [u8],
        theme: Theme,
        frame_width: usize,
        frame_height: usize,
    ) {
        if framebuffer.len() != frame_width * frame_height * 4 {
            return;
        }

        for sprite in query.sprites() {
            let start_x = sprite.x as usize;
            let start_y = sprite.y as usize;

            // Standard 8x8 sprite dimensions
            let end_x = (start_x + 8).min(frame_width);
            let end_y = (start_y + 8).min(frame_height);

            for y in start_y..end_y {
                for x in start_x..end_x {
                    let pixel_index = (y * frame_width + x) * 4;
                    // Apply theme to the single pixel slice.
                    // ThemeFilter processes chunks of 4.
                    let pixel_slice = &mut framebuffer[pixel_index..pixel_index + 4];
                    ThemeFilter::apply_theme(pixel_slice, theme);
                }
            }
        }
    }
}

#[cfg(all(test, feature = "nova"))]
mod tests {
    use super::*;
    use crate::Command;
    use crate::NesCore;

    #[test]
    fn test_spatial_themer() {
        let mut core = NesCore::new();

        let mut dummy_page = [0xff; 256];
        dummy_page[0] = 10; // Y
        dummy_page[3] = 10; // X
        core.load_cpu_bytes(0x0200, &dummy_page);
        core.write_cpu_bus(0x4014, 0x02);

        for _ in 0..180 {
            core.execute(Command::StepCpu).unwrap();
        }

        let query = OamSpatialQuery::new(&core);
        let mut framebuffer = vec![255; 256 * 240 * 4];

        SpatialThemer::apply_to_sprites(&query, &mut framebuffer, Theme::Grayscale, 256, 240);

        // Check pixel inside sprite 0 box (x=10, y=10)
        let inside_idx = (10 * 256 + 10) * 4;
        assert_eq!(framebuffer[inside_idx], framebuffer[inside_idx + 1]);
        assert_eq!(framebuffer[inside_idx + 1], framebuffer[inside_idx + 2]);

        // Check pixel outside sprite 0 box (x=20, y=20)
        let outside_idx = (20 * 256 + 20) * 4;
        assert_eq!(framebuffer[outside_idx], 255);
        assert_eq!(framebuffer[outside_idx + 1], 255);
    }
}
