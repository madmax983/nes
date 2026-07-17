//! Experimental tool for tracking sprite velocity over time.
//!
//! This module uses `OamSpatialQuery` to extract sprite positions frame by frame,
//! linking them by their OAM index to compute instantaneous velocity vectors (`dx`, `dy`).

#[cfg(feature = "nova")]
use crate::NesCore;
#[cfg(feature = "nova")]
use crate::experimental::oam_spatial_query::OamSpatialQuery;

#[cfg(feature = "nova")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Represents the velocity and position of a tracked sprite.
pub struct TrackedSprite {
    /// The index of the sprite in OAM (0..64).
    pub index: u8,
    /// The X coordinate of the left of the sprite (0..255).
    pub x: u8,
    /// The Y coordinate of the top of the sprite (0..255).
    pub y: u8,
    /// Change in X since the last frame.
    pub dx: i16,
    /// Change in Y since the last frame.
    pub dy: i16,
}

#[cfg(feature = "nova")]
#[derive(Debug, Clone)]
/// An engine that tracks sprite positions across frames to compute velocity.
pub struct SpriteVelocityTracker {
    last_positions: [Option<(u8, u8)>; 64],
    velocities: [Option<TrackedSprite>; 64],
}

#[cfg(feature = "nova")]
impl Default for SpriteVelocityTracker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(feature = "nova")]
impl SpriteVelocityTracker {
    /// Creates a new, empty tracker.
    #[must_use]
    pub fn new() -> Self {
        Self {
            last_positions: [None; 64],
            velocities: [None; 64],
        }
    }

    /// Evaluates the current core's OAM to calculate the new velocities.
    pub fn track(&mut self, core: &NesCore) {
        let query = OamSpatialQuery::new(core);

        for sprite in query.sprites() {
            let idx = sprite.index as usize;

            if let Some((last_x, last_y)) = self.last_positions[idx] {
                // Calculate velocity. We cast to i16 to handle wrapping/negative motion appropriately.
                // Assuming motion doesn't wrap around the entire screen in one frame.
                let mut dx = i16::from(sprite.x) - i16::from(last_x);
                let mut dy = i16::from(sprite.y) - i16::from(last_y);

                // Heuristic for screen wrap: if it moves > 128 pixels in one frame, it likely wrapped.
                if dx > 128 {
                    dx -= 256;
                } else if dx < -128 {
                    dx += 256;
                }

                if dy > 128 {
                    dy -= 256;
                } else if dy < -128 {
                    dy += 256;
                }

                self.velocities[idx] = Some(TrackedSprite {
                    index: sprite.index,
                    x: sprite.x,
                    y: sprite.y,
                    dx,
                    dy,
                });
            } else {
                // First time seeing this sprite, velocity is zero.
                self.velocities[idx] = Some(TrackedSprite {
                    index: sprite.index,
                    x: sprite.x,
                    y: sprite.y,
                    dx: 0,
                    dy: 0,
                });
            }

            self.last_positions[idx] = Some((sprite.x, sprite.y));
        }
    }

    /// Returns a list of currently active tracked sprites with their velocities.
    #[must_use]
    pub fn velocities(&self) -> Vec<TrackedSprite> {
        self.velocities.iter().filter_map(|&v| v).collect()
    }
}

#[cfg(all(test, feature = "nova"))]
mod tests {
    use super::*;

    #[test]
    fn test_velocity_tracking() {
        let mut core = NesCore::new();
        let mut tracker = SpriteVelocityTracker::new();

        // Frame 1
        let mut oam1 = [0xff; 256];
        oam1[0] = 50; // Y
        oam1[3] = 50; // X
        core.load_cpu_bytes(0x0200, &oam1);
        core.write_cpu_bus(0x4014, 0x02);
        for _ in 0..180 {
            let _ = core.execute(crate::Command::StepCpu);
        }

        tracker.track(&core);
        let vels = tracker.velocities();
        assert_eq!(vels[0].dx, 0);
        assert_eq!(vels[0].dy, 0);

        // Frame 2
        let mut oam2 = [0xff; 256];
        oam2[0] = 52; // Y moved by 2
        oam2[3] = 49; // X moved by -1
        core.load_cpu_bytes(0x0200, &oam2);
        core.write_cpu_bus(0x4014, 0x02);
        for _ in 0..180 {
            let _ = core.execute(crate::Command::StepCpu);
        }

        tracker.track(&core);
        let vels2 = tracker.velocities();
        assert_eq!(vels2[0].dx, -1);
        assert_eq!(vels2[0].dy, 2);
    }
}
