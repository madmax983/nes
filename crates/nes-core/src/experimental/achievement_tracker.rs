//! Experimental memory-based achievement tracking system.
//!
//! This module allows defining simple memory conditions (e.g., reaching a certain score,
//! collecting a specific item, or entering a particular level) and tracking their
//! completion state over time by evaluating the NES memory bus.

#[cfg(feature = "nova")]
use crate::NesCore;

#[cfg(feature = "nova")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// The type of condition to check in memory.
pub enum ConditionType {
    /// The memory value must equal this exact byte.
    Equals(u8),
    /// The memory value must be greater than this byte.
    GreaterThan(u8),
    /// The memory value must be less than this byte.
    LessThan(u8),
    /// Specific bits must be set (value & mask == mask).
    BitsSet(u8),
}

#[cfg(feature = "nova")]
#[derive(Debug, Clone, PartialEq, Eq)]
/// A single condition required for an achievement.
pub struct Condition {
    /// The 16-bit CPU memory address to inspect.
    pub address: u16,
    /// The condition the value at the address must meet.
    pub condition_type: ConditionType,
}

#[cfg(feature = "nova")]
impl Condition {
    /// Evaluates if the condition is met given the current memory value.
    pub fn is_met(&self, value: u8) -> bool {
        match self.condition_type {
            ConditionType::Equals(v) => value == v,
            ConditionType::GreaterThan(v) => value > v,
            ConditionType::LessThan(v) => value < v,
            ConditionType::BitsSet(mask) => (value & mask) == mask,
        }
    }
}

#[cfg(feature = "nova")]
#[derive(Debug, Clone, PartialEq, Eq)]
/// Represents a trackable achievement.
pub struct Achievement {
    /// A unique identifier for this achievement.
    pub id: std::string::String,
    /// A human-readable title.
    pub title: std::string::String,
    /// A description of how to unlock it.
    pub description: std::string::String,
    /// The conditions that must all be true simultaneously to unlock this achievement.
    pub conditions: std::vec::Vec<Condition>,
    /// Whether the achievement has been unlocked yet.
    pub is_unlocked: bool,
}

#[cfg(feature = "nova")]
impl Achievement {
    /// Creates a new, locked achievement.
    pub fn new(
        id: impl Into<std::string::String>,
        title: impl Into<std::string::String>,
        description: impl Into<std::string::String>,
        conditions: std::vec::Vec<Condition>,
    ) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            description: description.into(),
            conditions,
            is_unlocked: false,
        }
    }
}

#[cfg(feature = "nova")]
/// Tracks a set of achievements against a running NES core.
pub struct AchievementTracker {
    achievements: std::vec::Vec<Achievement>,
}

#[cfg(feature = "nova")]
impl Default for AchievementTracker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(feature = "nova")]
impl AchievementTracker {
    /// Creates a new, empty tracker.
    pub fn new() -> Self {
        Self {
            achievements: std::vec::Vec::new(),
        }
    }

    /// Adds an achievement to track.
    pub fn add_achievement(&mut self, achievement: Achievement) {
        self.achievements.push(achievement);
    }

    /// Evaluates all currently locked achievements against the NES memory state.
    /// Returns a list of the IDs of any newly unlocked achievements during this evaluation.
    pub fn evaluate(&mut self, core: &NesCore) -> std::vec::Vec<std::string::String> {
        let mut unlocked_now = std::vec::Vec::new();

        for achievement in &mut self.achievements {
            if achievement.is_unlocked {
                continue;
            }

            let mut all_met = true;
            for cond in &achievement.conditions {
                // We use core.read_memory to peek at the bus without side effects
                let val = core.read_memory(cond.address);
                if !cond.is_met(val) {
                    all_met = false;
                    break;
                }
            }

            if all_met {
                achievement.is_unlocked = true;
                unlocked_now.push(achievement.id.clone());
            }
        }

        unlocked_now
    }

    /// Returns a reference to the list of tracked achievements.
    pub fn achievements(&self) -> &[Achievement] {
        &self.achievements
    }
}

#[cfg(all(test, feature = "nova"))]
mod tests {
    use super::*;

    #[test]
    fn test_condition_evaluation() {
        let eq = Condition {
            address: 0x0000,
            condition_type: ConditionType::Equals(5),
        };
        assert!(eq.is_met(5));
        assert!(!eq.is_met(4));

        let gt = Condition {
            address: 0x0000,
            condition_type: ConditionType::GreaterThan(10),
        };
        assert!(gt.is_met(11));
        assert!(!gt.is_met(10));

        let lt = Condition {
            address: 0x0000,
            condition_type: ConditionType::LessThan(10),
        };
        assert!(lt.is_met(9));
        assert!(!lt.is_met(10));

        let bits = Condition {
            address: 0x0000,
            condition_type: ConditionType::BitsSet(0b1010_0000),
        };
        assert!(bits.is_met(0b1010_0000));
        assert!(bits.is_met(0b1111_1111));
        assert!(!bits.is_met(0b1000_0000));
    }

    #[test]
    fn test_achievement_tracker() {
        let mut core = NesCore::new();
        // Setup RAM so it behaves like RAM for our test reads.
        // We write directly to the CPU bus.
        core.write_cpu_bus(0x0010, 0x00);
        core.write_cpu_bus(0x0020, 0x00);

        let mut tracker = AchievementTracker::new();
        tracker.add_achievement(Achievement::new(
            "score_100",
            "High Score",
            "Reach 100 points.",
            vec![Condition {
                address: 0x0010,
                condition_type: ConditionType::Equals(100),
            }],
        ));

        // Evaluate when condition is not met
        let unlocked = tracker.evaluate(&core);
        assert!(unlocked.is_empty());
        assert!(!tracker.achievements()[0].is_unlocked);

        // Change memory so condition IS met
        core.write_cpu_bus(0x0010, 100);
        let unlocked = tracker.evaluate(&core);
        assert_eq!(unlocked, vec!["score_100"]);
        assert!(tracker.achievements()[0].is_unlocked);

        // Evaluate again, shouldn't trigger unlock again
        let unlocked_again = tracker.evaluate(&core);
        assert!(unlocked_again.is_empty());
    }
}
