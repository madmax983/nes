const CTRL_NMI_ENABLE: u8 = 0x80;
const STATUS_VBLANK: u8 = 0x80;

const DOTS_PER_SCANLINE: u16 = 341;
const SCANLINES_PER_FRAME: u16 = 262;
const VBLANK_SCANLINE: u16 = 241;
const PRE_RENDER_SCANLINE: u16 = 261;
const VBLANK_EDGE_DOT: u16 = 1;
const RENDER_MASK_BITS: u8 = 0x18;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PpuSnapshot {
    pub ctrl: u8,
    pub mask: u8,
    pub status: u8,
    pub cycle_in_frame: u32,
    pub scanline: u16,
    pub dot: u16,
    pub odd_frame: bool,
    pub frame_counter: u64,
    pub nmi_pending: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Ppu {
    ctrl: u8,
    mask: u8,
    status: u8,
    scanline: u16,
    dot: u16,
    odd_frame: bool,
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
            scanline: 0,
            dot: 0,
            odd_frame: false,
            frame_counter: 0,
            nmi_pending: false,
        }
    }

    pub fn reset(&mut self) {
        self.ctrl = 0;
        self.mask = 0;
        self.status = 0;
        self.scanline = 0;
        self.dot = 0;
        self.odd_frame = false;
        self.frame_counter = 0;
        self.nmi_pending = false;
    }

    pub fn restore(&mut self, snapshot: PpuSnapshot) {
        self.ctrl = snapshot.ctrl;
        self.mask = snapshot.mask;
        self.status = snapshot.status;
        self.scanline = snapshot.scanline;
        self.dot = snapshot.dot;
        self.odd_frame = snapshot.odd_frame;
        self.frame_counter = snapshot.frame_counter;
        self.nmi_pending = snapshot.nmi_pending;
    }

    #[must_use]
    pub fn snapshot(&self) -> PpuSnapshot {
        PpuSnapshot {
            ctrl: self.ctrl,
            mask: self.mask,
            status: self.status,
            cycle_in_frame: self.cycle_in_frame(),
            scanline: self.scanline,
            dot: self.dot,
            odd_frame: self.odd_frame,
            frame_counter: self.frame_counter,
            nmi_pending: self.nmi_pending,
        }
    }

    pub fn step_dot(&mut self) {
        if self.scanline == PRE_RENDER_SCANLINE
            && self.dot == DOTS_PER_SCANLINE - 2
            && self.odd_frame
            && self.rendering_enabled()
        {
            self.dot = 0;
            self.scanline = 0;
            self.frame_counter = self.frame_counter.saturating_add(1);
            self.odd_frame = !self.odd_frame;
            self.status &= !STATUS_VBLANK;
            return;
        }

        self.dot = self.dot.saturating_add(1);
        if self.dot >= DOTS_PER_SCANLINE {
            self.dot = 0;
            self.scanline = self.scanline.saturating_add(1);
            if self.scanline >= SCANLINES_PER_FRAME {
                self.scanline = 0;
                self.frame_counter = self.frame_counter.saturating_add(1);
                self.odd_frame = !self.odd_frame;
                self.status &= !STATUS_VBLANK;
            }
        }

        if self.scanline == VBLANK_SCANLINE && self.dot == VBLANK_EDGE_DOT {
            self.status |= STATUS_VBLANK;
            if self.ctrl & CTRL_NMI_ENABLE != 0 {
                self.nmi_pending = true;
            }
        }

        if self.scanline == PRE_RENDER_SCANLINE && self.dot == VBLANK_EDGE_DOT {
            self.status &= !STATUS_VBLANK;
        }
    }

    pub fn write_register(&mut self, register: u16, value: u8) {
        match register {
            0x2000 => {
                let nmi_before = self.ctrl & CTRL_NMI_ENABLE != 0;
                self.ctrl = value;
                let nmi_after = self.ctrl & CTRL_NMI_ENABLE != 0;
                if !nmi_before && nmi_after && self.status & STATUS_VBLANK != 0 {
                    self.nmi_pending = true;
                }
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
    pub fn scanline(&self) -> u16 {
        self.scanline
    }

    #[must_use]
    pub fn dot(&self) -> u16 {
        self.dot
    }

    #[must_use]
    pub fn cycle_in_frame(&self) -> u32 {
        (u32::from(self.scanline) * u32::from(DOTS_PER_SCANLINE)) + u32::from(self.dot)
    }

    #[must_use]
    pub fn take_nmi_pending(&mut self) -> bool {
        let pending = self.nmi_pending;
        self.nmi_pending = false;
        pending
    }

    #[must_use]
    fn rendering_enabled(&self) -> bool {
        self.mask & RENDER_MASK_BITS != 0
    }
}

impl Default for Ppu {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::{Ppu, STATUS_VBLANK};

    #[test]
    fn vblank_edges_are_dot_exact() {
        let mut ppu = Ppu::new();
        ppu.write_register(0x2000, 0x80); // enable NMI-on-vblank

        let mut saw_set = false;
        let mut saw_clear = false;
        for _ in 0..(341_u32 * 262 * 2) {
            let before = ppu.status() & STATUS_VBLANK != 0;
            ppu.step_dot();
            let after = ppu.status() & STATUS_VBLANK != 0;

            if !before && after {
                assert_eq!(ppu.scanline(), 241);
                assert_eq!(ppu.dot(), 1);
                assert!(ppu.take_nmi_pending());
                saw_set = true;
            }

            if before && !after {
                assert_eq!(ppu.scanline(), 261);
                assert_eq!(ppu.dot(), 1);
                saw_clear = true;
                break;
            }
        }

        assert!(saw_set);
        assert!(saw_clear);
    }

    #[test]
    fn odd_frame_shortens_by_one_dot_when_rendering_enabled() {
        let mut ppu = Ppu::new();
        ppu.write_register(0x2001, 0x18); // background + sprites enabled

        let mut frame_lengths = [0_u32; 2];
        let mut current = 0_u32;
        let mut frame_index = 0usize;

        while frame_index < 2 {
            ppu.step_dot();
            current = current.saturating_add(1);
            if ppu.frame_counter() == (frame_index as u64 + 1) {
                frame_lengths[frame_index] = current;
                current = 0;
                frame_index += 1;
            }
        }

        assert_eq!(frame_lengths[0], 341 * 262);
        assert_eq!(frame_lengths[1], 341 * 262 - 1);
    }
}
