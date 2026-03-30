#[cfg(feature = "nova")]
use font8x8::{BASIC_FONTS, UnicodeFonts};

#[cfg(feature = "nova")]
use nes_core::{FRAME_HEIGHT, FRAME_WIDTH};

#[cfg(feature = "nova")]
#[derive(Debug, Clone)]
pub struct MetricsOverlayData {
    pub emu_fps: f64,
    pub avg_render_ms: f64,
    pub late_frames: u64,
    pub audio_drops: u64,
    pub net_rtt_ms: f64,
    pub net_rollbacks: u64,
}

#[cfg(feature = "nova")]
pub struct MetricsOverlay {
    pub enabled: bool,
    data: Option<MetricsOverlayData>,
}

#[cfg(feature = "nova")]
impl MetricsOverlay {
    pub fn new(enabled: bool) -> Self {
        Self {
            enabled,
            data: None,
        }
    }

    pub fn update(&mut self, data: MetricsOverlayData) {
        self.data = Some(data);
    }

    pub fn render(&self, frame: &mut [u8]) {
        if !self.enabled {
            return;
        }
        let Some(data) = &self.data else {
            return;
        };

        let lines = [
            format!("FPS: {:.1}", data.emu_fps),
            format!("RND: {:.2}ms", data.avg_render_ms),
            format!("LATE: {}", data.late_frames),
            format!("ADRP: {}", data.audio_drops),
            format!("RTT: {:.1}ms", data.net_rtt_ms),
            format!("RLL: {}", data.net_rollbacks),
        ];

        let mut y = 8;
        for line in lines {
            self.draw_text(frame, 8, y, &line, [0, 255, 0, 255]);
            y += 10;
        }
    }

    fn draw_text(&self, frame: &mut [u8], x: usize, y: usize, text: &str, color: [u8; 4]) {
        let mut cur_x = x;
        for ch in text.chars() {
            if let Some(glyph) = BASIC_FONTS.get(ch) {
                for (r, row) in glyph.iter().enumerate() {
                    for c in 0..8 {
                        if (row & (1 << c)) != 0 {
                            let px = cur_x + c;
                            let py = y + r;
                            if px < FRAME_WIDTH && py < FRAME_HEIGHT {
                                let idx = (py * FRAME_WIDTH + px) * 4;
                                frame[idx] = color[0];
                                frame[idx + 1] = color[1];
                                frame[idx + 2] = color[2];
                                frame[idx + 3] = color[3];
                            }
                        }
                    }
                }
            }
            cur_x += 8;
        }
    }
}

#[cfg(all(test, feature = "nova"))]
mod tests {
    use super::*;

    #[test]
    fn overlay_renders_text_into_framebuffer() {
        let mut overlay = MetricsOverlay::new(true);
        overlay.update(MetricsOverlayData {
            emu_fps: 60.1,
            avg_render_ms: 2.5,
            late_frames: 0,
            audio_drops: 0,
            net_rtt_ms: 12.3,
            net_rollbacks: 5,
        });

        let mut frame = vec![0_u8; FRAME_WIDTH * FRAME_HEIGHT * 4];
        overlay.render(&mut frame);

        // Spot check a pixel that should be drawn (the first letter 'F')
        let mut has_drawn_pixel = false;
        for chunk in frame.chunks_exact(4) {
            if chunk == [0, 255, 0, 255] {
                has_drawn_pixel = true;
                break;
            }
        }
        assert!(
            has_drawn_pixel,
            "Overlay should have rendered visible pixels"
        );
    }

    #[test]
    fn overlay_does_not_render_when_disabled() {
        let mut overlay = MetricsOverlay::new(false);
        overlay.update(MetricsOverlayData {
            emu_fps: 60.1,
            avg_render_ms: 2.5,
            late_frames: 0,
            audio_drops: 0,
            net_rtt_ms: 12.3,
            net_rollbacks: 5,
        });

        let mut frame = vec![0_u8; FRAME_WIDTH * FRAME_HEIGHT * 4];
        overlay.render(&mut frame);

        let has_drawn_pixel = frame.chunks_exact(4).any(|c| c == [0, 255, 0, 255]);
        assert!(!has_drawn_pixel, "Overlay should not render when disabled");
    }
}
