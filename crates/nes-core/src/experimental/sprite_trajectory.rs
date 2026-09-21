//! Experimental sprite trajectory tracking for NES OAM.
//!
//! This module builds upon `OamSpatialQuery` to track the movement of sprites
//! across frames, calculating their velocity and predicting future positions.

#[cfg(feature = "nova")]
use crate::experimental::oam_spatial_query::OamSprite;
#[cfg(feature = "nova")]
use std::collections::HashMap;

#[cfg(feature = "nova")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Velocity {
    pub vx: i16,
    pub vy: i16,
}

#[cfg(feature = "nova")]
#[derive(Debug, Default, Clone)]
pub struct SpriteTrajectoryTracker {
    /// Maps OAM sprite index (0..64) to its last known position.
    last_positions: HashMap<u8, (u8, u8)>,
    /// Maps OAM sprite index to its current velocity.
    velocities: HashMap<u8, Velocity>,
}

#[cfg(feature = "nova")]
impl SpriteTrajectoryTracker {
    #[must_use]
    pub fn new() -> Self {
        Self {
            last_positions: HashMap::new(),
            velocities: HashMap::new(),
        }
    }

    /// Updates the tracker with the latest sprite positions.
    pub fn update(&mut self, current_sprites: &[OamSprite]) {
        for sprite in current_sprites {
            let current_pos = (sprite.x, sprite.y);

            if let Some(&last_pos) = self.last_positions.get(&sprite.index) {
                let vx = i16::from(current_pos.0) - i16::from(last_pos.0);
                let vy = i16::from(current_pos.1) - i16::from(last_pos.1);

                // Handle basic screen wrapping heuristically.
                // If velocity is massive, it probably wrapped or the sprite was reused.
                // For this prototype, we'll just track it raw.
                self.velocities.insert(sprite.index, Velocity { vx, vy });
            }

            self.last_positions.insert(sprite.index, current_pos);
        }
    }

    /// Predicts the future position of a sprite after a given number of frames.
    #[must_use]
    pub fn predict_position(&self, sprite_index: u8, frames_ahead: u32) -> Option<(u8, u8)> {
        let pos = self.last_positions.get(&sprite_index)?;
        let vel = self.velocities.get(&sprite_index)?;

        let frames = i32::try_from(frames_ahead).unwrap_or(i32::MAX);

        let next_x = i32::from(pos.0) + (i32::from(vel.vx) * frames);
        let next_y = i32::from(pos.1) + (i32::from(vel.vy) * frames);

        // Clamp to screen boundaries roughly (NES screen is 256x240, but sprites use 255 wrapping)
        let clamped_x = next_x.clamp(0, 255) as u8;
        let clamped_y = next_y.clamp(0, 255) as u8;

        Some((clamped_x, clamped_y))
    }
}

#[cfg(all(test, feature = "nova"))]
mod tests {
    use super::*;
    use crate::experimental::oam_spatial_query::OamSprite;

    #[test]
    fn test_velocity_tracking() {
        let mut tracker = SpriteTrajectoryTracker::new();

        let frame1 = vec![OamSprite {
            index: 0,
            x: 10,
            y: 20,
            tile_id: 0,
            attributes: 0,
        }];
        tracker.update(&frame1);

        // No velocity on first frame
        assert!(!tracker.velocities.contains_key(&0));

        let frame2 = vec![OamSprite {
            index: 0,
            x: 15,
            y: 22,
            tile_id: 0,
            attributes: 0,
        }];
        tracker.update(&frame2);

        let vel = tracker.velocities.get(&0).unwrap();
        assert_eq!(vel.vx, 5);
        assert_eq!(vel.vy, 2);
    }

    #[test]
    fn test_prediction_with_clamping() {
        let mut tracker = SpriteTrajectoryTracker::new();

        tracker.update(&[OamSprite {
            index: 1,
            x: 200,
            y: 100,
            tile_id: 0,
            attributes: 0,
        }]);
        tracker.update(&[OamSprite {
            index: 1,
            x: 210,
            y: 90,
            tile_id: 0,
            attributes: 0,
        }]); // vx = 10, vy = -10

        // Predict 1 frame ahead
        let p1 = tracker.predict_position(1, 1).unwrap();
        assert_eq!(p1, (220, 80));

        // Predict out of bounds (should clamp)
        let p2 = tracker.predict_position(1, 10).unwrap();
        // x: 210 + 100 = 310 -> clamps to 255
        // y: 90 - 100 = -10 -> clamps to 0
        assert_eq!(p2, (255, 0));
    }
}
