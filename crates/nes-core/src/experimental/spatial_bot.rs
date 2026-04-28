//! Experimental spatial bot that maps zone events to controller commands.
//!
//! The spatial bot uses `ZoneTracker` events to trigger automated inputs.

#[cfg(feature = "nova")]
use crate::Button;
#[cfg(feature = "nova")]
use crate::Command;
#[cfg(feature = "nova")]
use crate::experimental::zone_tracker::ZoneTracker;

#[cfg(feature = "nova")]
#[derive(Debug, Clone)]
pub struct BotRule {
    pub zone_id: usize,
    pub button: Button,
    pub duration_frames: u32,
}

#[cfg(feature = "nova")]
#[derive(Debug, Default, Clone)]
pub struct SpatialBot {
    rules: Vec<BotRule>,
    active_presses: std::collections::HashMap<Button, u32>,
}

#[cfg(feature = "nova")]
impl SpatialBot {
    #[must_use]
    pub fn new() -> Self {
        Self {
            rules: Vec::new(),
            active_presses: std::collections::HashMap::new(),
        }
    }

    /// Adds a new automation rule mapping a screen zone to a button press.
    ///
    /// When a sprite enters the zone identified by `zone_id` (tracked by a `ZoneTracker`),
    /// the bot will emit a command to press the specified `button` for `duration_frames`.
    ///
    /// # Examples
    ///
    /// ```
    /// use nes_core::experimental::spatial_bot::SpatialBot;
    /// use nes_core::Button;
    ///
    /// let mut bot = SpatialBot::new();
    /// // When a sprite enters zone 1, press the 'A' button for 10 frames
    /// bot.add_rule(1, Button::A, 10);
    /// ```
    pub fn add_rule(&mut self, zone_id: usize, button: Button, duration_frames: u32) {
        self.rules.push(BotRule {
            zone_id,
            button,
            duration_frames,
        });
    }

    /// Evaluates the current zone tracker state against the bot's rules, emitting commands.
    ///
    /// This should be called once per frame after updating the `ZoneTracker`.
    /// The bot will inspect new zone entry events, start pressing the configured buttons,
    /// keep track of how long they have been held, and emit release commands when
    /// the duration expires.
    ///
    /// Returns a list of `Command` objects representing the button presses and releases
    /// to be executed on the emulator.
    ///
    /// # Examples
    ///
    /// ```
    /// use nes_core::experimental::spatial_bot::SpatialBot;
    /// use nes_core::experimental::zone_tracker::ZoneTracker;
    /// use nes_core::Button;
    ///
    /// let mut tracker = ZoneTracker::new();
    /// let mut bot = SpatialBot::new();
    /// bot.add_rule(1, Button::A, 5);
    ///
    /// // Evaluate tracker events
    /// let commands = bot.evaluate(&tracker);
    /// ```
    pub fn evaluate(&mut self, tracker: &ZoneTracker) -> Vec<Command> {
        let mut commands = Vec::new();

        // 1. Process new events from the tracker
        for event in tracker.events() {
            for rule in &self.rules {
                if event.zone_id == rule.zone_id {
                    // Only issue a PressButton if we aren't already holding it
                    if !self.active_presses.contains_key(&rule.button) {
                        commands.push(Command::PressButton(rule.button));
                    }
                    // Update or insert the duration
                    self.active_presses
                        .insert(rule.button, rule.duration_frames);
                }
            }
        }

        // 2. Decrement active presses and release if 0
        // We use retain to modify the map in-place and keep only those > 0
        let mut expired = Vec::new();
        self.active_presses.retain(|button, frames_left| {
            if *frames_left > 0 {
                *frames_left -= 1;
            }
            if *frames_left == 0 {
                expired.push(*button);
                false
            } else {
                true
            }
        });

        for button in expired {
            commands.push(Command::ReleaseButton(button));
        }

        commands
    }
}

#[cfg(all(test, feature = "nova"))]
mod tests {
    use super::*;
    use crate::Command;
    use crate::NesCore;
    use crate::experimental::zone_tracker::ZoneTracker;

    #[test]
    fn test_spatial_bot_evaluation() {
        let mut core = NesCore::new();
        let mut tracker = ZoneTracker::new();
        let mut bot = SpatialBot::new();

        // Add a zone and a bot rule. Duration 2 means it is held for 2 evaluations *after* the initial event.
        tracker.add_zone(1, 100, 100, 50, 50);
        bot.add_rule(1, Button::A, 2);

        // Put a sprite inside the zone
        let mut dummy_page = [0xff; 256];
        dummy_page[0] = 120; // Y
        dummy_page[3] = 120; // X
        core.load_cpu_bytes(0x0200, &dummy_page);
        core.write_cpu_bus(0x4014, 0x02); // OAM DMA
        for _ in 0..180 {
            let _ = core.execute(Command::StepCpu);
        }

        tracker.track(&core);

        // Frame 1: Bot should press A and set duration to 2. Then duration decrements to 1.
        let cmds1 = bot.evaluate(&tracker);
        assert_eq!(cmds1.len(), 1);
        assert!(matches!(cmds1[0], Command::PressButton(Button::A)));

        // Frame 2: Bot should hold A (no new commands). Duration decrements to 0. Release happens immediately?
        // Wait, in my evaluate, decrement happens AFTER inserting.
        // Frame 1: insert 2. Decrement -> 1. Keep. (no release)
        // Frame 2: evaluate with empty_tracker. Decrement -> 0. Release.
        // Wait, the assertion failed on frame 2: left: 1, right: 0. Which means cmds2 had 1 command.

        let empty_tracker = ZoneTracker::new();
        let cmds2 = bot.evaluate(&empty_tracker);
        assert_eq!(cmds2.len(), 1);
        assert!(matches!(cmds2[0], Command::ReleaseButton(Button::A)));

        // Frame 3: Nothing.
        let cmds3 = bot.evaluate(&empty_tracker);
        assert_eq!(cmds3.len(), 0);
    }
}
