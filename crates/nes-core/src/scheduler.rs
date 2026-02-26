#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SchedulerSnapshot {
    pub cpu_cycles: u64,
    pub ppu_cycles: u64,
    pub apu_cycles: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Scheduler {
    cpu_cycles: u64,
    ppu_cycles: u64,
    apu_cycles: u64,
}

impl Scheduler {
    #[must_use]
    pub fn new() -> Self {
        Self {
            cpu_cycles: 0,
            ppu_cycles: 0,
            apu_cycles: 0,
        }
    }

    pub fn step_cpu_cycle(&mut self) {
        self.cpu_cycles = self.cpu_cycles.saturating_add(1);
    }

    pub fn step_ppu_cycle(&mut self) {
        self.ppu_cycles = self.ppu_cycles.saturating_add(1);
    }

    pub fn step_apu_cycle(&mut self) {
        self.apu_cycles = self.apu_cycles.saturating_add(1);
    }

    #[must_use]
    pub fn snapshot(&self) -> SchedulerSnapshot {
        SchedulerSnapshot {
            cpu_cycles: self.cpu_cycles,
            ppu_cycles: self.ppu_cycles,
            apu_cycles: self.apu_cycles,
        }
    }

    pub fn restore(&mut self, snapshot: SchedulerSnapshot) {
        self.cpu_cycles = snapshot.cpu_cycles;
        self.ppu_cycles = snapshot.ppu_cycles;
        self.apu_cycles = snapshot.apu_cycles;
    }

    pub fn reset(&mut self) {
        self.cpu_cycles = 0;
        self.ppu_cycles = 0;
        self.apu_cycles = 0;
    }

    #[must_use]
    pub fn total_cycles(&self) -> u64 {
        self.cpu_cycles
    }

    #[must_use]
    pub fn cpu_cycles(&self) -> u64 {
        self.cpu_cycles
    }

    #[must_use]
    pub fn ppu_cycles(&self) -> u64 {
        self.ppu_cycles
    }

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
