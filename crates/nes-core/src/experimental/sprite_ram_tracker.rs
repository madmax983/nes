//! Experimental tool for automatically discovering the CPU RAM addresses of on-screen sprites.
//!
//! `SpriteRamTracker` combines `OamSpatialQuery` and `CheatFinder` to map physical screen
//! entities back to their underlying memory variables (e.g., finding the Player X/Y coordinates
//! in RAM). This enables automated reverse-engineering of game state.

#[cfg(feature = "nova")]
use crate::NesCore;
#[cfg(feature = "nova")]
use crate::experimental::cheat_finder::{CheatFinder, MemoryCandidate};
#[cfg(feature = "nova")]
use crate::experimental::oam_spatial_query::OamSpatialQuery;

#[cfg(feature = "nova")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// The specific attribute of a sprite to track in CPU RAM.
pub enum SpriteAttribute {
    /// The X coordinate on the screen.
    X,
    /// The Y coordinate on the screen.
    Y,
    /// The visual Tile ID.
    TileId,
    /// The sprite attributes (palette, flip, priority).
    Attributes,
}

#[cfg(feature = "nova")]
/// Automatically isolates the CPU RAM address that corresponds to a specific sprite's attribute.
///
/// By calling `track` every frame, the tracker will follow the sprite in OAM and continuously
/// filter the `CheatFinder` for memory addresses that match the sprite's current value.
/// Over time, this narrows down the candidates to the exact memory location.
pub struct SpriteRamTracker {
    sprite_index: u8,
    attribute: SpriteAttribute,
    finder: CheatFinder,
    initialized: bool,
}

#[cfg(feature = "nova")]
impl SpriteRamTracker {
    /// Creates a new `SpriteRamTracker` for a given OAM sprite index and attribute.
    ///
    /// ## Examples
    ///
    /// ```
    /// # use nes_core::experimental::sprite_ram_tracker::{SpriteRamTracker, SpriteAttribute};
    /// // Track the X coordinate of Sprite 0 (often the player character)
    /// let tracker = SpriteRamTracker::new(0, SpriteAttribute::X);
    /// ```
    #[must_use]
    pub fn new(sprite_index: u8, attribute: SpriteAttribute) -> Self {
        Self {
            sprite_index,
            attribute,
            finder: CheatFinder::new(),
            initialized: false,
        }
    }

    /// Evaluates the current state of the `NesCore` and filters memory candidates.
    ///
    /// This should be called periodically (e.g., once per frame) while the sprite is active.
    pub fn track(&mut self, core: &NesCore) {
        let query = OamSpatialQuery::new(core);
        let sprites = query.sprites();

        if (self.sprite_index as usize) < sprites.len() {
            let sprite = sprites[self.sprite_index as usize];
            let target_value = match self.attribute {
                SpriteAttribute::X => sprite.x,
                SpriteAttribute::Y => sprite.y,
                SpriteAttribute::TileId => sprite.tile_id,
                SpriteAttribute::Attributes => sprite.attributes,
            };

            if !self.initialized {
                self.finder.reset_search(core);
                self.initialized = true;
            }

            self.finder.filter_eq(core, target_value);
        }
    }

    /// Returns the current list of memory candidates that might hold the attribute.
    #[must_use]
    pub fn candidates(&self) -> &[MemoryCandidate] {
        self.finder.candidates()
    }
}

#[cfg(all(test, feature = "nova"))]
mod tests {
    use super::*;
    use crate::Command;

    #[test]
    fn test_sprite_ram_tracker() {
        let mut core = NesCore::new();

        // Put Sprite 0 at X=50, Y=100
        let mut dummy_page = [0xff; 256];
        dummy_page[0] = 100; // Y
        dummy_page[3] = 50; // X

        // Pretend game logic stores X at 0x0010 and Y at 0x0011, and DMA copies from 0x0200.
        core.load_cpu_bytes(0x0010, &[50]);
        core.load_cpu_bytes(0x0011, &[100]);
        core.load_cpu_bytes(0x0200, &dummy_page);

        // Trigger OAM DMA
        core.write_cpu_bus(0x4014, 0x02);
        for _ in 0..600 {
            let _ = core.execute(Command::StepCpu);
        }

        let mut tracker_x = SpriteRamTracker::new(0, SpriteAttribute::X);
        tracker_x.track(&core);

        let candidates = tracker_x.candidates();
        assert!(candidates.iter().any(|c| c.addr == 0x0010));
        assert!(candidates.iter().any(|c| c.addr == 0x0203));

        // Move the sprite to X=60
        dummy_page[3] = 60;
        core.load_cpu_bytes(0x0010, &[60]);
        core.load_cpu_bytes(0x0200, &dummy_page);

        core.write_cpu_bus(0x4014, 0x02);
        for _ in 0..600 {
            let _ = core.execute(Command::StepCpu);
        }

        tracker_x.track(&core);

        let candidates2 = tracker_x.candidates();
        assert!(candidates2.iter().any(|c| c.addr == 0x0010));
        assert!(candidates2.iter().any(|c| c.addr == 0x0203));
    }
}
