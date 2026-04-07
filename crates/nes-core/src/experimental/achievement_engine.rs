//! Experimental achievement engine.
//!
//! Evaluates memory states to unlock defined achievements.

#[cfg(feature = "nova")]
use crate::NesCore;

#[cfg(feature = "nova")]
#[derive(Debug, Clone, PartialEq, Eq)]
/// A condition that must be met in memory to unlock an achievement.
pub enum Condition {
    /// Memory at address equals the specified value.
    MemoryEquals {
        /// The physical memory address in the NES RAM.
        addr: u16,
        /// The byte value that must be present.
        value: u8,
    },
    /// Memory at address is strictly greater than the specified value.
    MemoryGreaterThan {
        /// The physical memory address in the NES RAM.
        addr: u16,
        /// The byte value that the memory must exceed.
        value: u8,
    },
    /// Memory at address is strictly less than the specified value.
    MemoryLessThan {
        /// The physical memory address in the NES RAM.
        addr: u16,
        /// The byte value that the memory must be strictly less than.
        value: u8,
    },
}

#[cfg(feature = "nova")]
impl Condition {
    /// Evaluates if the condition is met given the current state of the core.
    #[must_use]
    pub fn is_met(&self, core: &NesCore) -> bool {
        match *self {
            Condition::MemoryEquals { addr, value } => core.read_memory(addr) == value,
            Condition::MemoryGreaterThan { addr, value } => core.read_memory(addr) > value,
            Condition::MemoryLessThan { addr, value } => core.read_memory(addr) < value,
        }
    }
}

#[cfg(feature = "nova")]
#[derive(Debug, Clone, PartialEq, Eq)]
/// Defines an achievement with a set of conditions that must all be met.
pub struct Achievement {
    /// Unique identifier for the achievement.
    pub id: String,
    /// Display name of the achievement.
    pub name: String,
    /// Description of how to unlock the achievement.
    pub description: String,
    /// The conditions that must all be true to unlock.
    pub conditions: Vec<Condition>,
}

#[cfg(feature = "nova")]
impl Achievement {
    /// Creates a new achievement.
    pub fn new(
        id: impl Into<String>,
        name: impl Into<String>,
        description: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            description: description.into(),
            conditions: Vec::new(),
        }
    }

    /// Adds a condition to the achievement.
    #[must_use]
    pub fn with_condition(mut self, condition: Condition) -> Self {
        self.conditions.push(condition);
        self
    }

    /// Evaluates if all conditions for this achievement are met.
    #[must_use]
    pub fn is_met(&self, core: &NesCore) -> bool {
        if self.conditions.is_empty() {
            return false;
        }
        self.conditions.iter().all(|c| c.is_met(core))
    }
}

#[cfg(feature = "nova")]
#[derive(Debug, Default, Clone)]
/// Tracks registered achievements and evaluates them against the emulator state.
pub struct AchievementEngine {
    achievements: Vec<Achievement>,
    unlocked_ids: Vec<String>,
}

#[cfg(feature = "nova")]
impl AchievementEngine {
    /// Creates a new, empty achievement engine.
    #[must_use]
    pub fn new() -> Self {
        Self {
            achievements: Vec::new(),
            unlocked_ids: Vec::new(),
        }
    }

    /// Registers a new achievement to be tracked.
    pub fn add_achievement(&mut self, achievement: Achievement) {
        self.achievements.push(achievement);
    }

    /// Evaluates all locked achievements against the current core state.
    /// Returns a list of newly unlocked achievement IDs.
    pub fn evaluate(&mut self, core: &NesCore) -> Vec<String> {
        let mut newly_unlocked = Vec::new();

        for achievement in &self.achievements {
            if !self.unlocked_ids.contains(&achievement.id) && achievement.is_met(core) {
                self.unlocked_ids.push(achievement.id.clone());
                newly_unlocked.push(achievement.id.clone());
            }
        }

        newly_unlocked
    }

    /// Returns the IDs of all unlocked achievements.
    #[must_use]
    pub fn unlocked_ids(&self) -> &[String] {
        &self.unlocked_ids
    }
}

#[cfg(all(test, feature = "nova"))]
mod tests {
    use super::*;
    use crate::NesCore;

    #[test]
    fn test_condition_evaluation() {
        let mut core = NesCore::new();
        core.load_cpu_bytes(0x0100, &[10]); // load value 10 at 0x0100

        let cond_eq = Condition::MemoryEquals {
            addr: 0x0100,
            value: 10,
        };
        let cond_gt = Condition::MemoryGreaterThan {
            addr: 0x0100,
            value: 5,
        };
        let cond_lt = Condition::MemoryLessThan {
            addr: 0x0100,
            value: 20,
        };

        assert!(cond_eq.is_met(&core));
        assert!(cond_gt.is_met(&core));
        assert!(cond_lt.is_met(&core));

        let cond_not_eq = Condition::MemoryEquals {
            addr: 0x0100,
            value: 9,
        };
        assert!(!cond_not_eq.is_met(&core));
    }

    #[test]
    fn test_achievement_engine() {
        let mut core = NesCore::new();
        core.load_cpu_bytes(0x0050, &[0]); // Initial state: 0 coins

        let mut engine = AchievementEngine::new();
        engine.add_achievement(
            Achievement::new("coin_collector", "Coin Collector", "Collect 100 coins")
                .with_condition(Condition::MemoryEquals {
                    addr: 0x0050,
                    value: 100,
                }),
        );

        // Initially not unlocked
        assert!(engine.evaluate(&core).is_empty());
        assert!(engine.unlocked_ids().is_empty());

        // Still not unlocked
        core.load_cpu_bytes(0x0050, &[50]);
        assert!(engine.evaluate(&core).is_empty());

        // Now unlocked
        core.load_cpu_bytes(0x0050, &[100]);
        let newly_unlocked = engine.evaluate(&core);
        assert_eq!(newly_unlocked.len(), 1);
        assert_eq!(newly_unlocked[0], "coin_collector");

        // Already unlocked, shouldn't unlock again
        assert!(engine.evaluate(&core).is_empty());
        assert_eq!(engine.unlocked_ids().len(), 1);
    }

    #[test]
    fn test_achievement_with_no_conditions() {
        let core = NesCore::new();
        let achievement = Achievement::new("no_cond", "No Conditions", "Empty");
        // An achievement with no conditions should never be met by default
        assert!(!achievement.is_met(&core));
    }
}
