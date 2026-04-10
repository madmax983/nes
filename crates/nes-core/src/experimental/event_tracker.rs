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
    pub fn new() -> Self {
        Self {
            triggers: Vec::new(),
            events: Vec::new(),
        }
    }

    pub fn add_trigger(&mut self, addr: u16, expected_value: u8) {
        self.triggers.push(Trigger {
            addr,
            expected_value,
        });
    }

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
