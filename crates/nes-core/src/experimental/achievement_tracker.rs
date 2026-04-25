//! Experimental achievement tracking system.
//!
//! This module provides a way to define and track memory-based conditions
//! to unlock achievements during gameplay, similar to RetroAchievements.

#[cfg(feature = "nova")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Condition {
    /// The memory at the given address must exactly equal the specified value.
    MemoryEquals { address: u16, value: u8 },
    /// The memory at the given address must be strictly greater than the specified value.
    MemoryGreaterThan { address: u16, value: u8 },
    /// The memory at the given address must be strictly less than the specified value.
    MemoryLessThan { address: u16, value: u8 },
}

#[cfg(feature = "nova")]
#[derive(Debug, Clone)]
pub struct Achievement {
    /// Unique identifier for the achievement.
    pub id: String,
    /// Display name of the achievement.
    pub name: String,
    /// Description of how to unlock it.
    pub description: String,
    /// Conditions that must all be true to unlock.
    pub conditions: Vec<Condition>,
    /// Whether the achievement has been unlocked.
    pub unlocked: bool,
}

#[cfg(feature = "nova")]
impl Achievement {
    /// Creates a new achievement.
    pub fn new(
        id: impl Into<String>,
        name: impl Into<String>,
        description: impl Into<String>,
        conditions: Vec<Condition>,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            description: description.into(),
            conditions,
            unlocked: false,
        }
    }
}

#[cfg(feature = "nova")]
#[derive(Default)]
pub struct AchievementTracker {
    achievements: Vec<Achievement>,
}

#[cfg(feature = "nova")]
impl AchievementTracker {
    /// Creates a new, empty achievement tracker.
    pub fn new() -> Self {
        Self {
            achievements: Vec::new(),
        }
    }

    /// Adds a new achievement to track.
    pub fn add_achievement(&mut self, achievement: Achievement) {
        self.achievements.push(achievement);
    }

    /// Evaluates all locked achievements against the provided memory read function.
    /// Returns a list of newly unlocked achievements this tick.
    pub fn evaluate<F>(&mut self, mut memory_reader: F) -> Vec<Achievement>
    where
        F: FnMut(u16) -> u8,
    {
        let mut newly_unlocked = Vec::new();

        for achievement in &mut self.achievements {
            if achievement.unlocked {
                continue;
            }

            if achievement.conditions.is_empty() {
                continue;
            }

            let all_conditions_met = achievement.conditions.iter().all(|cond| match cond {
                Condition::MemoryEquals { address, value } => memory_reader(*address) == *value,
                Condition::MemoryGreaterThan { address, value } => memory_reader(*address) > *value,
                Condition::MemoryLessThan { address, value } => memory_reader(*address) < *value,
            });

            if all_conditions_met {
                achievement.unlocked = true;
                newly_unlocked.push(achievement.clone());
            }
        }

        newly_unlocked
    }
}

#[cfg(all(test, feature = "nova"))]
mod tests {
    use super::*;

    #[test]
    fn test_achievement_unlock() {
        let mut tracker = AchievementTracker::new();

        tracker.add_achievement(Achievement::new(
            "smb_coin",
            "First Coin!",
            "Collect your first coin in SMB",
            vec![Condition::MemoryGreaterThan {
                address: 0x075E,
                value: 0,
            }],
        ));

        // Fake memory: 0 coins
        let mut memory = vec![0u8; 2048];
        let unlocked = tracker.evaluate(|addr| memory[addr as usize]);
        assert!(unlocked.is_empty());

        // Fake memory: 1 coin
        memory[0x075E] = 1;
        let unlocked = tracker.evaluate(|addr| memory[addr as usize]);
        assert_eq!(unlocked.len(), 1);
        assert_eq!(unlocked[0].id, "smb_coin");

        // Subsequent evaluations shouldn't re-trigger
        let unlocked_again = tracker.evaluate(|addr| memory[addr as usize]);
        assert!(unlocked_again.is_empty());
    }

    #[test]
    fn test_multiple_conditions() {
        let mut tracker = AchievementTracker::new();

        tracker.add_achievement(Achievement::new(
            "boss_defeat",
            "Boss Defeated",
            "Defeat the boss without taking damage",
            vec![
                Condition::MemoryEquals {
                    address: 0x0010,
                    value: 0,
                }, // Boss HP = 0
                Condition::MemoryEquals {
                    address: 0x0020,
                    value: 3,
                }, // Player HP = 3 (Max)
            ],
        ));

        let mut memory = vec![0u8; 2048];
        memory[0x0010] = 5; // Boss HP
        memory[0x0020] = 3; // Player HP

        // Boss still alive
        let unlocked = tracker.evaluate(|addr| memory[addr as usize]);
        assert!(unlocked.is_empty());

        // Boss defeated but player took damage
        memory[0x0010] = 0;
        memory[0x0020] = 2;
        let unlocked = tracker.evaluate(|addr| memory[addr as usize]);
        assert!(unlocked.is_empty());

        // Boss defeated AND player at max HP
        memory[0x0020] = 3;
        let unlocked = tracker.evaluate(|addr| memory[addr as usize]);
        assert_eq!(unlocked.len(), 1);
    }
}
