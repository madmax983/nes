//! Experimental Cinematic Spotlight filter.

#[cfg(feature = "nova")]
use crate::experimental::oam_spatial_query::OamSprite;
#[cfg(feature = "nova")]
use crate::experimental::theme_filter::Theme;
#[cfg(feature = "nova")]
use crate::experimental::theme_filter::ThemeFilter;

#[cfg(feature = "nova")]
pub struct Spotlight {
    pub center_x: u16,
    pub center_y: u16,
    pub radius: f32,
}

#[cfg(feature = "nova")]
impl Spotlight {
    pub fn from_sprite(sprite: &OamSprite, radius: f32) -> Self {
        Self {
            center_x: sprite.x as u16 + 4,
            center_y: sprite.y as u16 + 4,
            radius,
        }
    }
}

#[cfg(feature = "nova")]
pub struct CinematicSpotlight;

#[cfg(feature = "nova")]
impl CinematicSpotlight {
    pub fn apply(
        framebuffer: &mut [u8],
        width: usize,
        height: usize,
        spotlight: &Spotlight,
        theme: Theme,
    ) {
        let mut themed_buffer = framebuffer.to_vec();
        ThemeFilter::apply_theme(&mut themed_buffer, theme);

        for y in 0..height {
            for x in 0..width {
                let dx = (x as f32) - (spotlight.center_x as f32);
                let dy = (y as f32) - (spotlight.center_y as f32);
                let dist_sq = dx * dx + dy * dy;

                if dist_sq > spotlight.radius * spotlight.radius {
                    let idx = (y * width + x) * 4;
                    framebuffer[idx] = themed_buffer[idx];
                    framebuffer[idx + 1] = themed_buffer[idx + 1];
                    framebuffer[idx + 2] = themed_buffer[idx + 2];
                }
            }
        }
    }
}

#[cfg(all(test, feature = "nova"))]
mod tests {
    use super::*;

    #[test]
    fn spotlight_from_sprite() {
        let sprite = OamSprite {
            index: 0,
            x: 100,
            y: 50,
            tile_id: 0,
            attributes: 0,
        };
        let spot = Spotlight::from_sprite(&sprite, 30.0);
        // 100 + 4 (half sprite width) = 104
        assert_eq!(spot.center_x, 104);
        assert_eq!(spot.center_y, 54);
        assert_eq!(spot.radius, 30.0);
    }

    #[test]
    fn spotlight_applies_theme_outside_radius() {
        let mut frame = vec![200, 100, 50, 255, 200, 100, 50, 255]; // 2 pixels
        let spot = Spotlight {
            center_x: 0,
            center_y: 0,
            radius: 0.5,
        }; // Pixel 0 is inside, Pixel 1 is outside

        CinematicSpotlight::apply(&mut frame, 2, 1, &spot, Theme::Grayscale);

        // Pixel 0 (inside) should be untouched
        assert_eq!(frame[0], 200);
        assert_eq!(frame[1], 100);
        assert_eq!(frame[2], 50);

        // Pixel 1 (outside) should be Grayscale
        assert_eq!(frame[4], frame[5]); // R == G
        assert_eq!(frame[5], frame[6]); // G == B
        assert!(frame[4] != 200);
    }
}
