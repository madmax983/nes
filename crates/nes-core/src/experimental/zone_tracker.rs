//! Experimental spatial zone tracking for NES sprites.
//!
//! This module allows users to define rectangular "zones" on the screen and track
//! when sprites enter those zones. It builds upon `OamSpatialQuery` to monitor
//! sprite movement over time, generating events when a sprite triggers a zone.

use alloc::vec::Vec;

#[cfg(feature = "nova")]
use crate::NesCore;
#[cfg(feature = "nova")]
use crate::experimental::oam_spatial_query::{OamSpatialQuery, OamSprite};

#[cfg(feature = "nova")]
#[derive(Debug, Clone, PartialEq, Eq)]
/// An event triggered when a sprite enters a defined zone.
pub struct ZoneEvent {
    /// The ID of the zone that was triggered.
    pub zone_id: usize,
    /// The sprite that entered the zone.
    pub sprite: OamSprite,
}

#[cfg(feature = "nova")]
#[derive(Debug, Clone)]
/// A defined rectangular area on the screen to monitor.
pub struct Zone {
    /// The unique identifier for this zone.
    pub id: usize,
    /// The left X coordinate of the zone bounding box.
    pub x: u8,
    /// The top Y coordinate of the zone bounding box.
    pub y: u8,
    /// The width of the zone in pixels.
    pub w: u8,
    /// The height of the zone in pixels.
    pub h: u8,
}

#[cfg(feature = "nova")]
#[derive(Debug, Default, Clone)]
/// Tracks sprites entering defined screen zones.
pub struct ZoneTracker {
    zones: Vec<Zone>,
    events: Vec<ZoneEvent>,
    /// Keeps track of which sprites were in which zones last frame to trigger only on entry.
    /// Maps zone_id to a list of sprite indices.
    active_sprites: std::collections::HashMap<usize, Vec<u8>>,
}

#[cfg(feature = "nova")]
impl ZoneTracker {
    /// Creates a new, empty `ZoneTracker`.
    #[must_use]
    pub fn new() -> Self {
        Self {
            zones: Vec::new(),
            events: Vec::new(),
            active_sprites: std::collections::HashMap::new(),
        }
    }

    /// Adds a new rectangular zone to monitor.
    pub fn add_zone(&mut self, id: usize, x: u8, y: u8, w: u8, h: u8) {
        self.zones.push(Zone { id, x, y, w, h });
        self.active_sprites.insert(id, Vec::new());
    }

    /// Evaluates the current frame's OAM against registered zones.
    ///
    /// Generates `ZoneEvent`s for any sprites that have newly entered a zone.
    pub fn track(&mut self, core: &NesCore) {
        let oam_query = OamSpatialQuery::new(core);

        for zone in &self.zones {
            let overlapping_sprites = oam_query.query_rect(zone.x, zone.y, zone.w, zone.h);

            let mut current_sprite_indices = Vec::with_capacity(overlapping_sprites.len());
            let previously_active = self.active_sprites.get(&zone.id);

            for sprite in overlapping_sprites {
                current_sprite_indices.push(sprite.index);

                // If the sprite wasn't in the zone last frame, it's a new entry event
                if previously_active.is_none_or(|prev| !prev.contains(&sprite.index)) {
                    self.events.push(ZoneEvent {
                        zone_id: zone.id,
                        sprite,
                    });
                }
            }

            // Update active sprites for this zone for the next frame
            self.active_sprites.insert(zone.id, current_sprite_indices);
        }
    }

    /// Returns a slice of all recorded `ZoneEvent`s.
    #[must_use]
    pub fn events(&self) -> &[ZoneEvent] {
        &self.events
    }
}

#[cfg(all(test, feature = "nova"))]
mod tests {
    use super::*;
    use crate::Command;

    #[test]
    fn test_zone_tracker_entry_events() {
        let mut core = NesCore::new();
        let mut tracker = ZoneTracker::new();

        // Add a zone in the middle of the screen
        tracker.add_zone(1, 100, 100, 50, 50);

        // Put a sprite outside the zone
        let mut dummy_page = [0xff; 256];
        dummy_page[0] = 50; // Y
        dummy_page[3] = 50; // X
        core.load_cpu_bytes(0x0200, &dummy_page);
        core.write_cpu_bus(0x4014, 0x02); // OAM DMA
        for _ in 0..180 {
            let _ = core.execute(Command::StepCpu);
        }

        tracker.track(&core);
        assert_eq!(
            tracker.events().len(),
            0,
            "No event should be triggered when sprite is outside"
        );

        // Move sprite into the zone
        dummy_page[0] = 120; // Y
        dummy_page[3] = 120; // X
        core.load_cpu_bytes(0x0200, &dummy_page);
        core.write_cpu_bus(0x4014, 0x02); // OAM DMA
        for _ in 0..180 {
            let _ = core.execute(Command::StepCpu);
        }

        tracker.track(&core);
        assert_eq!(
            tracker.events().len(),
            1,
            "Event should be triggered on entry"
        );
        assert_eq!(tracker.events()[0].zone_id, 1);
        assert_eq!(tracker.events()[0].sprite.index, 0);

        // Keep sprite in the zone
        core.write_cpu_bus(0x4014, 0x02); // OAM DMA
        for _ in 0..180 {
            let _ = core.execute(Command::StepCpu);
        }

        tracker.track(&core);
        assert_eq!(
            tracker.events().len(),
            1,
            "No new event should be triggered if already inside"
        );

        // Move sprite out, then back in
        dummy_page[0] = 50; // Y
        dummy_page[3] = 50; // X
        core.load_cpu_bytes(0x0200, &dummy_page);
        core.write_cpu_bus(0x4014, 0x02); // OAM DMA
        for _ in 0..180 {
            let _ = core.execute(Command::StepCpu);
        }
        tracker.track(&core); // Out

        dummy_page[0] = 120; // Y
        dummy_page[3] = 120; // X
        core.load_cpu_bytes(0x0200, &dummy_page);
        core.write_cpu_bus(0x4014, 0x02); // OAM DMA
        for _ in 0..180 {
            let _ = core.execute(Command::StepCpu);
        }
        tracker.track(&core); // In

        assert_eq!(
            tracker.events().len(),
            2,
            "Second event should be triggered on re-entry"
        );
    }
}
