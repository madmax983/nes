//! Experimental memory watcher for triggering events when specific memory locations meet conditions.
//!
//! The memory watcher allows developers and users to set "watchpoints" on the NES memory bus.
//! This is useful for debugging, reverse engineering, or creating custom triggers (e.g.,
//! achievements or custom scripting based on game state).

#[cfg(feature = "nova")]
use crate::NesCore;

#[cfg(feature = "nova")]
#[derive(Debug, Clone, PartialEq, Eq)]
/// Defines the condition under which a watchpoint triggers.
pub enum WatchCondition {
    /// Triggers when the value at the address changes from its previous state.
    Changed,
    /// Triggers when the value at the address exactly equals the specified value.
    Equals(u8),
    /// Triggers when the value at the address is strictly greater than the specified value.
    GreaterThan(u8),
    /// Triggers when the value at the address is strictly less than the specified value.
    LessThan(u8),
}

#[cfg(feature = "nova")]
#[derive(Debug, Clone, PartialEq, Eq)]
/// A configured watchpoint targeting a specific memory address.
pub struct Watchpoint {
    /// The physical memory address in the NES RAM/bus to watch.
    pub addr: u16,
    /// The condition that must be met for this watchpoint to trigger.
    pub condition: WatchCondition,
    /// A human-readable label for this watchpoint.
    pub name: String,
    /// The last known value at this address, used for change detection.
    pub last_value: u8,
}

#[cfg(feature = "nova")]
/// A system for tracking and evaluating memory watchpoints against an emulator instance.
pub struct MemoryWatcher {
    watchpoints: Vec<Watchpoint>,
}

#[cfg(feature = "nova")]
impl Default for MemoryWatcher {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(feature = "nova")]
impl MemoryWatcher {
    /// Creates a new, empty memory watcher.
    #[must_use]
    pub fn new() -> Self {
        Self {
            watchpoints: Vec::new(),
        }
    }

    /// Adds a new watchpoint to the watcher.
    pub fn add_watchpoint(
        &mut self,
        addr: u16,
        condition: WatchCondition,
        name: String,
        initial_value: u8,
    ) {
        self.watchpoints.push(Watchpoint {
            addr,
            condition,
            name,
            last_value: initial_value,
        });
    }

    /// Evaluates all watchpoints against the current state of the provided `NesCore`.
    ///
    /// Returns a list of watchpoints that have been triggered during this check.
    /// Internal states (like `last_value`) are updated automatically.
    #[must_use]
    pub fn check(&mut self, core: &NesCore) -> Vec<Watchpoint> {
        let mut triggered = Vec::new();
        for wp in &mut self.watchpoints {
            let current_value = core.read_memory(wp.addr);
            let is_triggered = match wp.condition {
                WatchCondition::Changed => current_value != wp.last_value,
                WatchCondition::Equals(v) => current_value == v,
                WatchCondition::GreaterThan(v) => current_value > v,
                WatchCondition::LessThan(v) => current_value < v,
            };

            if is_triggered {
                triggered.push(wp.clone());
            }
            wp.last_value = current_value;
        }
        triggered
    }
}

#[cfg(test)]
#[cfg(feature = "nova")]
mod tests {
    use super::*;
    use crate::NesCore;

    #[test]
    fn test_memory_watcher() {
        let mut core = NesCore::new();

        let mut watcher = MemoryWatcher::new();
        watcher.add_watchpoint(
            0x0000,
            WatchCondition::Changed,
            "Zero Page First Byte".to_string(),
            0,
        );
        watcher.add_watchpoint(
            0x0001,
            WatchCondition::Equals(42),
            "Meaning of Life".to_string(),
            0,
        );

        let triggered = watcher.check(&core);
        assert!(triggered.is_empty());

        core.write_cpu_bus(0x0000, 10);

        let triggered = watcher.check(&core);
        assert_eq!(triggered.len(), 1);
        assert_eq!(triggered[0].name, "Zero Page First Byte");

        core.write_cpu_bus(0x0001, 42);
        let triggered = watcher.check(&core);
        assert_eq!(triggered.len(), 1);
        assert_eq!(triggered[0].name, "Meaning of Life");
    }
}
