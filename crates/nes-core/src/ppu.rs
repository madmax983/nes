const CTRL_NMI_ENABLE: u8 = 0x80;
const STATUS_VBLANK: u8 = 0x80;

const PPU_CYCLES_PER_FRAME: u32 = 341 * 262;
const VBLANK_START_CYCLE: u32 = 341 * 241;
const PRE_RENDER_START_CYCLE: u32 = 341 * 261;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PpuSnapshot {
    pub ctrl: u8,
    pub mask: u8,
    pub status: u8,
    pub cycle_in_frame: u32,
    pub frame_counter: u64,
    pub nmi_pending: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Ppu {
    ctrl: u8,
    mask: u8,
    status: u8,
    cycle_in_frame: u32,
    frame_counter: u64,
    nmi_pending: bool,
}

impl Ppu {
    #[must_use]
    pub fn new() -> Self {
        Self {
            ctrl: 0,
            mask: 0,
            status: 0,
            cycle_in_frame: 0,
            frame_counter: 0,
            nmi_pending: false,
        }
    }

    pub fn reset(&mut self) {
        self.ctrl = 0;
        self.mask = 0;
        self.status = 0;
        self.cycle_in_frame = 0;
        self.frame_counter = 0;
        self.nmi_pending = false;
    }

    pub fn restore(&mut self, snapshot: PpuSnapshot) {
        self.ctrl = snapshot.ctrl;
        self.mask = snapshot.mask;
        self.status = snapshot.status;
        self.cycle_in_frame = snapshot.cycle_in_frame;
        self.frame_counter = snapshot.frame_counter;
        self.nmi_pending = snapshot.nmi_pending;
    }

    #[must_use]
    pub fn snapshot(&self) -> PpuSnapshot {
        PpuSnapshot {
            ctrl: self.ctrl,
            mask: self.mask,
            status: self.status,
            cycle_in_frame: self.cycle_in_frame,
            frame_counter: self.frame_counter,
            nmi_pending: self.nmi_pending,
        }
    }

    pub fn step_cycles(&mut self, cycles: u64) {
        for _ in 0..cycles {
            self.cycle_in_frame = self.cycle_in_frame.saturating_add(1);

            if self.cycle_in_frame == VBLANK_START_CYCLE {
                self.status |= STATUS_VBLANK;
                if self.ctrl & CTRL_NMI_ENABLE != 0 {
                    self.nmi_pending = true;
                }
            } else if self.cycle_in_frame == PRE_RENDER_START_CYCLE {
                self.status &= !STATUS_VBLANK;
            }

            if self.cycle_in_frame >= PPU_CYCLES_PER_FRAME {
                self.cycle_in_frame = 0;
                self.frame_counter = self.frame_counter.saturating_add(1);
                self.status &= !STATUS_VBLANK;
            }
        }
    }

    pub fn write_register(&mut self, register: u16, value: u8) {
        match register {
            0x2000 => {
                self.ctrl = value;
            }
            0x2001 => {
                self.mask = value;
            }
            0x2002 => {
                self.status = value;
            }
            _ => {}
        }
    }

    #[must_use]
    pub fn ctrl(&self) -> u8 {
        self.ctrl
    }

    #[must_use]
    pub fn mask(&self) -> u8 {
        self.mask
    }

    #[must_use]
    pub fn status(&self) -> u8 {
        self.status
    }

    #[must_use]
    pub fn frame_counter(&self) -> u64 {
        self.frame_counter
    }

    #[must_use]
    pub fn take_nmi_pending(&mut self) -> bool {
        let pending = self.nmi_pending;
        self.nmi_pending = false;
        pending
    }
}

impl Default for Ppu {
    fn default() -> Self {
        Self::new()
    }
}
