#[cfg(feature = "nova")]
use nes_core::{CoreQuery, NesCore, QueryResult};
#[cfg(feature = "nova")]
use nes_desktop::overlay::draw_text;

const COLOR_TEXT: [u8; 4] = [235, 239, 247, 255];
const COLOR_STATUS: [u8; 4] = [207, 173, 91, 255];
const LINE_HEIGHT: usize = 10;
const PANEL_PADDING: usize = 12;

#[cfg(feature = "nova")]
pub struct CpuDebugger {
    is_open: bool,
}

#[cfg(feature = "nova")]
impl Default for CpuDebugger {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(feature = "nova")]
impl CpuDebugger {
    pub fn new() -> Self {
        Self { is_open: false }
    }

    pub fn toggle(&mut self) -> bool {
        self.is_open = !self.is_open;
        self.is_open
    }

    pub fn is_open(&self) -> bool {
        self.is_open
    }

    pub fn draw_debugger(&self, core: &NesCore, frame: &mut [u8], frame_width: usize) {
        if !self.is_open {
            return;
        }

        let snapshot = if let QueryResult::Registers(r) = core.query(CoreQuery::Registers) {
            *r
        } else {
            return;
        };

        let pc = snapshot.pc;
        let a = snapshot.a;
        let x = snapshot.x;
        let y = snapshot.y;
        let sp = snapshot.sp;
        let status = snapshot.status;

        // Dim background logic
        for pixel in frame.chunks_exact_mut(4) {
            pixel[0] = pixel[0].saturating_sub(60);
            pixel[1] = pixel[1].saturating_sub(60);
            pixel[2] = pixel[2].saturating_sub(60);
        }

        let mut text_y = PANEL_PADDING;
        let text_x = PANEL_PADDING;

        draw_text(
            frame,
            frame_width,
            text_x,
            text_y,
            "HOMEBREW DEBUGGER",
            COLOR_TEXT,
        );
        text_y = text_y.saturating_add(LINE_HEIGHT + 2);

        let reg_str = format!(
            "PC:{:04X} A:{:02X} X:{:02X} Y:{:02X} SP:{:02X} P:{:02X}",
            pc, a, x, y, sp, status
        );
        draw_text(
            frame,
            frame_width,
            text_x,
            text_y,
            &reg_str,
            [255, 255, 0, 255],
        );
        text_y = text_y.saturating_add(LINE_HEIGHT + 4);

        draw_text(
            frame,
            frame_width,
            text_x,
            text_y,
            "Controls: I=Step Cpu, E=Step Frame",
            [200, 200, 200, 255],
        );
        text_y = text_y.saturating_add(LINE_HEIGHT + 4);

        // Simple memory hex dump near PC
        draw_text(
            frame,
            frame_width,
            text_x,
            text_y,
            "Memory view (Zero Page):",
            COLOR_TEXT,
        );
        text_y = text_y.saturating_add(LINE_HEIGHT);

        let start_addr = 0x0000;
        for i in 0..4 {
            let line_addr = start_addr + (i * 8) as u16;
            let mut line = format!("{:04X}:", line_addr);
            for j in 0..8 {
                let addr = line_addr.wrapping_add(j);
                if let QueryResult::Memory(val) = core.query(CoreQuery::Memory(addr)) {
                    line.push_str(&format!(" {:02X}", val));
                }
            }
            draw_text(frame, frame_width, text_x, text_y, &line, COLOR_STATUS);
            text_y = text_y.saturating_add(LINE_HEIGHT);
        }

        text_y = text_y.saturating_add(LINE_HEIGHT);
        draw_text(
            frame,
            frame_width,
            text_x,
            text_y,
            "Memory view (near PC):",
            COLOR_TEXT,
        );
        text_y = text_y.saturating_add(LINE_HEIGHT);

        let pc_start = pc.saturating_sub(8);
        for i in 0..4 {
            let line_addr = pc_start.wrapping_add((i * 8) as u16);
            let mut line = format!("{:04X}:", line_addr);
            for j in 0..8 {
                let addr = line_addr.wrapping_add(j);
                if let QueryResult::Memory(val) = core.query(CoreQuery::Memory(addr)) {
                    line.push_str(&format!(" {:02X}", val));
                }
            }
            draw_text(frame, frame_width, text_x, text_y, &line, COLOR_STATUS);
            text_y = text_y.saturating_add(LINE_HEIGHT);
        }
    }
}
