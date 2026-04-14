//! Experimental memory event tracking to trigger callbacks when specific conditions occur.
//!
//! This module allows users to define custom triggers (e.g., when a memory address changes or reaches a certain value)
//! and records the events.

#[cfg(feature = "nova")]
use crate::NesCore;

#[cfg(feature = "nova")]
#[derive(Debug, Clone)]
pub struct Event {
    pub addr: u16,
    pub value: u8,
}

#[cfg(feature = "nova")]
#[derive(Debug, Clone)]
pub struct Trigger {
    pub addr: u16,
    pub expected_value: u8,
}

#[cfg(feature = "nova")]
#[derive(Debug, Default, Clone)]
pub struct EventTracker {
    triggers: Vec<Trigger>,
    events: Vec<Event>,
}

#[cfg(feature = "nova")]
impl EventTracker {
    /// Creates a new, empty [`EventTracker`].
    ///
    /// The tracker initializes with no active triggers and an empty event log.
    /// You must add triggers using [`add_trigger`](EventTracker::add_trigger) before calling [`track`](EventTracker::track).
    ///
    /// ## Examples
    ///
    /// ```
    /// # use nes_core::experimental::event_tracker::EventTracker;
    /// let tracker = EventTracker::new();
    /// assert!(tracker.events().is_empty());
    /// ```
    pub fn new() -> Self {
        Self {
            triggers: Vec::new(),
            events: Vec::new(),
        }
    }

    /// Adds a new condition to monitor memory values.
    ///
    /// When [`track`](EventTracker::track) is called, it will check if the memory at `addr` matches `expected_value`.
    /// If it does, a new [`Event`] is pushed into the tracker's event log.
    ///
    /// ## Examples
    ///
    /// ```
    /// # use nes_core::experimental::event_tracker::EventTracker;
    /// let mut tracker = EventTracker::new();
    /// tracker.add_trigger(0x0200, 42); // Monitor address 0x0200 for the value 42
    /// ```
    pub fn add_trigger(&mut self, addr: u16, expected_value: u8) {
        self.triggers.push(Trigger {
            addr,
            expected_value,
        });
    }

    /// Evaluates all registered triggers against the current memory state of the provided [`NesCore`].
    ///
    /// For every trigger that matches, an [`Event`] is generated and appended to the event log.
    /// This is typically called once per frame or immediately after specific CPU execution steps.
    ///
    /// ## Examples
    ///
    /// ```
    /// # use nes_core::experimental::event_tracker::EventTracker;
    /// # use nes_core::NesCore;
    /// let mut core = NesCore::new();
    /// let mut tracker = EventTracker::new();
    /// tracker.add_trigger(0x0200, 42);
    ///
    /// // ... after some execution that sets 0x0200 to 42 ...
    /// core.load_cpu_bytes(0x0200, &[42]);
    /// tracker.track(&core);
    /// assert_eq!(tracker.events().len(), 1);
    /// ```
    pub fn track(&mut self, core: &NesCore) {
        for trigger in &self.triggers {
            let value = core.read_memory(trigger.addr);
            if value == trigger.expected_value {
                self.events.push(Event {
                    addr: trigger.addr,
                    value,
                });
            }
        }
    }

    /// Returns a slice of all recorded memory [`Event`]s.
    ///
    /// The log grows indefinitely as [`track`](EventTracker::track) is called and triggers are met.
    ///
    /// ## Examples
    ///
    /// ```
    /// # use nes_core::experimental::event_tracker::EventTracker;
    /// let tracker = EventTracker::new();
    /// assert_eq!(tracker.events().len(), 0);
    /// ```
    pub fn events(&self) -> &[Event] {
        &self.events
    }
}

#[cfg(all(test, feature = "nova"))]
mod tests {
    use super::*;
    use crate::Command;
    use crate::NesCore;

    #[test]
    fn test_event_tracker() {
        let mut core = NesCore::new();
        let mut tracker = EventTracker::new();

        // Track a specific memory address change
        tracker.add_trigger(0x0200, 42);

        core.load_cpu_bytes(0x0200, &[42]);
        let _ = core.execute(Command::StepFrame);

        tracker.track(&core);

        assert_eq!(tracker.events().len(), 1);
        assert_eq!(tracker.events()[0].addr, 0x0200);
        assert_eq!(tracker.events()[0].value, 42);
    }
}
