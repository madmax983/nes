//! Global hardware cycle counters.
//!
//! The scheduler tracks aggregate CPU/PPU/APU cycle counts. It does not
//! schedule events directly; instead it acts as a deterministic counter source
//! for stepping logic, timing assertions, and state snapshots.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
/// Serializable scheduler snapshot.
pub struct SchedulerSnapshot {
    /// Total CPU cycles elapsed.
    pub cpu_cycles: u64,
    /// Total PPU cycles elapsed.
    pub ppu_cycles: u64,
    /// Total APU cycles elapsed.
    pub apu_cycles: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Live cycle counters for the running core.
pub struct Scheduler {
    cpu_cycles: u64,
    ppu_cycles: u64,
    apu_cycles: u64,
}

impl Scheduler {
    /// Creates a zeroed scheduler.
    #[must_use]
    pub fn new() -> Self {
        Self {
            cpu_cycles: 0,
            ppu_cycles: 0,
            apu_cycles: 0,
        }
    }

    /// Advances CPU cycles by one tick.
    pub fn step_cpu_cycle(&mut self) {
        self.cpu_cycles = self.cpu_cycles.wrapping_add(1);
    }

    /// Advances PPU cycles by one tick.
    pub fn step_ppu_cycle(&mut self) {
        self.ppu_cycles = self.ppu_cycles.wrapping_add(1);
    }

    /// Advances APU cycles by one tick.
    pub fn step_apu_cycle(&mut self) {
        self.apu_cycles = self.apu_cycles.wrapping_add(1);
    }

    /// Returns a copyable snapshot of all counters.
    #[must_use]
    pub fn snapshot(&self) -> SchedulerSnapshot {
        SchedulerSnapshot {
            cpu_cycles: self.cpu_cycles,
            ppu_cycles: self.ppu_cycles,
            apu_cycles: self.apu_cycles,
        }
    }

    /// Restores counters from a snapshot.
    pub fn restore(&mut self, snapshot: SchedulerSnapshot) {
        self.cpu_cycles = snapshot.cpu_cycles;
        self.ppu_cycles = snapshot.ppu_cycles;
        self.apu_cycles = snapshot.apu_cycles;
    }

    /// Clears all counters to zero.
    pub fn reset(&mut self) {
        self.cpu_cycles = 0;
        self.ppu_cycles = 0;
        self.apu_cycles = 0;
    }

    /// Returns total CPU cycles (the canonical timeline counter).
    #[must_use]
    pub fn total_cycles(&self) -> u64 {
        self.cpu_cycles
    }

    /// Returns CPU cycles.
    #[must_use]
    pub fn cpu_cycles(&self) -> u64 {
        self.cpu_cycles
    }

    /// Returns PPU cycles.
    #[must_use]
    pub fn ppu_cycles(&self) -> u64 {
        self.ppu_cycles
    }

    /// Returns APU cycles.
    #[must_use]
    pub fn apu_cycles(&self) -> u64 {
        self.apu_cycles
    }
}

impl Default for Scheduler {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::{Scheduler, SchedulerSnapshot};

    #[test]
    fn should_return_correct_default_scheduler() {
        let scheduler = Scheduler::default();
        assert_eq!(scheduler.cpu_cycles(), 0);
        assert_eq!(scheduler.ppu_cycles(), 0);
        assert_eq!(scheduler.apu_cycles(), 0);
        assert_eq!(scheduler.total_cycles(), 0);
    }

    #[test]
    fn should_save_and_restore_scheduler_snapshot() {
        let mut scheduler = Scheduler::new();
        scheduler.step_cpu_cycle();
        scheduler.step_ppu_cycle();
        scheduler.step_ppu_cycle();
        scheduler.step_apu_cycle();
        scheduler.step_apu_cycle();
        scheduler.step_apu_cycle();

        let snapshot = scheduler.snapshot();
        assert_eq!(snapshot.cpu_cycles, 1);
        assert_eq!(snapshot.ppu_cycles, 2);
        assert_eq!(snapshot.apu_cycles, 3);

        let mut scheduler2 = Scheduler::new();
        scheduler2.restore(snapshot);
        assert_eq!(scheduler2.cpu_cycles(), 1);
        assert_eq!(scheduler2.ppu_cycles(), 2);
        assert_eq!(scheduler2.apu_cycles(), 3);
    }

    #[test]
    fn should_reset_scheduler_counters() {
        let mut scheduler = Scheduler::new();
        scheduler.step_cpu_cycle();
        scheduler.step_ppu_cycle();
        scheduler.step_apu_cycle();

        assert_eq!(scheduler.cpu_cycles(), 1);
        assert_eq!(scheduler.ppu_cycles(), 1);
        assert_eq!(scheduler.apu_cycles(), 1);

        scheduler.reset();

        assert_eq!(scheduler.cpu_cycles(), 0);
        assert_eq!(scheduler.ppu_cycles(), 0);
        assert_eq!(scheduler.apu_cycles(), 0);
    }

    #[test]
    fn wrap_behavior_preserves_phase_instead_of_saturating() {
        let mut scheduler = Scheduler::new();
        scheduler.restore(SchedulerSnapshot {
            cpu_cycles: u64::MAX,
            ppu_cycles: u64::MAX,
            apu_cycles: u64::MAX,
        });

        scheduler.step_cpu_cycle();
        scheduler.step_ppu_cycle();
        scheduler.step_apu_cycle();

        assert_eq!(scheduler.cpu_cycles(), 0);
        assert_eq!(scheduler.ppu_cycles(), 0);
        assert_eq!(scheduler.apu_cycles(), 0);
    }

    #[test]
    fn restore_near_wrap_then_step_advances_and_wraps() {
        let mut scheduler = Scheduler::new();
        scheduler.restore(SchedulerSnapshot {
            cpu_cycles: u64::MAX - 1,
            ppu_cycles: 41,
            apu_cycles: 99,
        });

        scheduler.step_cpu_cycle();
        assert_eq!(scheduler.cpu_cycles(), u64::MAX);
        scheduler.step_cpu_cycle();
        assert_eq!(scheduler.cpu_cycles(), 0);

        scheduler.step_ppu_cycle();
        scheduler.step_apu_cycle();
        assert_eq!(scheduler.ppu_cycles(), 42);
        assert_eq!(scheduler.apu_cycles(), 100);
    }

    #[test]
    fn test_cpu_ppu_apu_getters() {
        let mut scheduler = Scheduler::new();
        scheduler.step_cpu_cycle();
        scheduler.step_ppu_cycle();
        scheduler.step_ppu_cycle();
        scheduler.step_apu_cycle();
        scheduler.step_apu_cycle();
        scheduler.step_apu_cycle();

        assert_eq!(scheduler.total_cycles(), 1);
        assert_eq!(scheduler.cpu_cycles(), 1);
        assert_eq!(scheduler.ppu_cycles(), 2);
        assert_eq!(scheduler.apu_cycles(), 3);
    }
}
