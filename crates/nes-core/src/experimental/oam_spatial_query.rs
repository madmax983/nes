//! Experimental spatial query capabilities for NES OAM (Object Attribute Memory).
//!
//! This module provides tools to decode the raw 256-byte OAM memory into structured
//! sprites and perform spatial queries (such as bounding-box collision detection)
//! against them. This is useful for AI, TAS tools, or cheat finders that need to
//! reason about physical entity placement on screen without analyzing the framebuffer.

use alloc::vec::Vec;

#[cfg(feature = "nova")]
use crate::NesCore;

#[cfg(feature = "nova")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Represents a decoded sprite from the NES OAM buffer.
pub struct OamSprite {
    /// The index of the sprite in OAM (0..64).
    pub index: u8,
    /// The Y coordinate of the top of the sprite (0..255).
    pub y: u8,
    /// The tile index number (0..255).
    pub tile_id: u8,
    /// The attribute flags (palette, flipping, priority).
    pub attributes: u8,
    /// The X coordinate of the left of the sprite (0..255).
    pub x: u8,
}

#[cfg(feature = "nova")]
impl OamSprite {
    /// Decodes a single sprite from the raw 256-byte OAM buffer.
    #[must_use]
    pub fn from_oam(index: u8, oam: &[u8; 256]) -> Self {
        let base = (index as usize) * 4;
        Self {
            index,
            y: oam[base],
            tile_id: oam[base + 1],
            attributes: oam[base + 2],
            x: oam[base + 3],
        }
    }

    /// Checks if this sprite's 8x8 bounding box overlaps with a given rectangle.
    #[must_use]
    pub fn overlaps(&self, rect_x: u8, rect_y: u8, rect_w: u8, rect_h: u8) -> bool {
        let sprite_right = self.x.saturating_add(8);
        let sprite_bottom = self.y.saturating_add(8);
        let rect_right = rect_x.saturating_add(rect_w);
        let rect_bottom = rect_y.saturating_add(rect_h);

        self.x < rect_right
            && sprite_right > rect_x
            && self.y < rect_bottom
            && sprite_bottom > rect_y
    }
}

#[cfg(feature = "nova")]
#[derive(Debug, Clone)]
/// An engine for decoding and querying OAM data spatially.
pub struct OamSpatialQuery {
    sprites: Vec<OamSprite>,
}

#[cfg(feature = "nova")]
impl OamSpatialQuery {
    /// Extracts all 64 sprites from the current OAM buffer.
    #[must_use]
    pub fn new(core: &NesCore) -> Self {
        let mut oam = [0u8; 256];
        for i in 0..=255 {
            oam[i as usize] = core.ppu_oam_byte(i);
        }

        let mut sprites = Vec::with_capacity(64);
        for i in 0..64 {
            sprites.push(OamSprite::from_oam(i, &oam));
        }

        Self { sprites }
    }

    /// Returns a list of sprites that overlap the given rectangle.
    #[must_use]
    pub fn query_rect(&self, x: u8, y: u8, w: u8, h: u8) -> Vec<OamSprite> {
        self.sprites
            .iter()
            .copied()
            .filter(|s| s.overlaps(x, y, w, h))
            .collect()
    }

    /// Returns all extracted sprites.
    #[must_use]
    pub fn sprites(&self) -> &[OamSprite] {
        &self.sprites
    }
}

#[cfg(all(test, feature = "nova"))]
mod tests {
    use super::*;

    #[test]
    fn test_oam_sprite_overlaps() {
        let sprite = OamSprite {
            index: 0,
            y: 10,
            tile_id: 0,
            attributes: 0,
            x: 20,
        };

        // Complete overlap
        assert!(sprite.overlaps(20, 10, 8, 8));

        // Partial overlap
        assert!(sprite.overlaps(24, 14, 8, 8));

        // Just touching boundaries (should not overlap)
        assert!(!sprite.overlaps(28, 10, 8, 8));
        assert!(!sprite.overlaps(20, 18, 8, 8));
        assert!(!sprite.overlaps(12, 10, 8, 8));
        assert!(!sprite.overlaps(20, 2, 8, 8));

        // Completely disjoint
        assert!(!sprite.overlaps(0, 0, 5, 5));
    }

    #[test]
    fn test_oam_spatial_query() {
        let mut core = NesCore::new();
        // The NES OAM is populated via DMA. We'll simulate OAM DMA to populate our mock.
        // OAM DMA copies 256 bytes from CPU memory page X to OAM.
        // We will put sprite data at 0x0200.
        let mut dummy_page = [0xff; 256]; // Fill with 0xFF (often hides sprites offscreen)

        // Sprite 0 at x=10, y=10
        dummy_page[0] = 10; // Y
        dummy_page[3] = 10; // X

        // Sprite 1 at x=50, y=50
        dummy_page[4] = 50; // Y
        dummy_page[7] = 50; // X

        // Sprite 63 (last sprite) at x=99, y=99
        dummy_page[252] = 99; // Y
        dummy_page[255] = 99; // X

        core.load_cpu_bytes(0x0200, &dummy_page);

        // Trigger OAM DMA from page 0x02
        core.write_cpu_bus(0x4014, 0x02);

        // Step the core enough to run the DMA (513-514 cycles roughly)
        for _ in 0..180 {
            let _ = core.execute(crate::Command::StepCpu);
        }

        let query = OamSpatialQuery::new(&core);

        // Query around Sprite 0
        let overlaps = query.query_rect(8, 8, 5, 5);
        assert_eq!(overlaps.len(), 1);
        assert_eq!(overlaps[0].index, 0);

        // Query area with no sprites
        let no_overlaps = query.query_rect(150, 150, 10, 10);
        assert_eq!(no_overlaps.len(), 0);

        // Ensure sprite 63 is parsed completely (especially X coord at byte 255)
        let last_sprite = query.sprites()[63];
        assert_eq!(last_sprite.x, 99);
        assert_eq!(last_sprite.y, 99);
    }
}
