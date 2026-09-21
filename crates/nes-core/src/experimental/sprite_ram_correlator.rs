//! Experimental tool to correlate OAM sprite coordinates with CPU RAM addresses.

use crate::NesCore;
use crate::experimental::oam_spatial_query::OamSprite;

/// Scans CPU work RAM to find addresses matching given OAM sprite coordinates.
///
/// `SpriteRamCorrelator` helps reverse-engineer games by linking visual data (sprites)
/// to internal game state (RAM). If a sprite is drawn at X=120, Y=150, this tool
/// scans the 2KB work RAM to find addresses holding 120 and 150, potentially revealing
/// where the game stores player or enemy positions.
pub struct SpriteRamCorrelator;

impl SpriteRamCorrelator {
    /// Finds addresses in CPU RAM that exactly match the sprite's X coordinate.
    ///
    /// Only the first 2KB of CPU RAM ($0000-$07FF) are scanned.
    #[must_use]
    pub fn find_x_candidates(core: &NesCore, sprite: &OamSprite) -> Vec<u16> {
        let mut candidates = Vec::new();
        let ram = core.cpu_snapshot().work_ram;
        for (addr, &val) in ram.iter().enumerate() {
            if val == sprite.x {
                candidates.push(addr as u16);
            }
        }
        candidates
    }

    /// Finds addresses in CPU RAM that exactly match the sprite's Y coordinate.
    ///
    /// Only the first 2KB of CPU RAM ($0000-$07FF) are scanned.
    #[must_use]
    pub fn find_y_candidates(core: &NesCore, sprite: &OamSprite) -> Vec<u16> {
        let mut candidates = Vec::new();
        let ram = core.cpu_snapshot().work_ram;
        for (addr, &val) in ram.iter().enumerate() {
            if val == sprite.y {
                candidates.push(addr as u16);
            }
        }
        candidates
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::NesCore;
    use crate::experimental::oam_spatial_query::OamSprite;

    #[test]
    fn test_correlate_sprite_coordinates() {
        let mut core = NesCore::new();
        // Inject some values into RAM
        core.write_cpu_bus(0x0042, 120); // Player X
        core.write_cpu_bus(0x0043, 150); // Player Y
        core.write_cpu_bus(0x0100, 120); // Coincidental X match

        let sprite = OamSprite {
            index: 0,
            x: 120,
            y: 150,
            tile_id: 1,
            attributes: 0,
        };

        let x_cands = SpriteRamCorrelator::find_x_candidates(&core, &sprite);
        let y_cands = SpriteRamCorrelator::find_y_candidates(&core, &sprite);

        assert!(x_cands.contains(&0x0042));
        assert!(x_cands.contains(&0x0100));
        assert!(y_cands.contains(&0x0043));
    }
}
