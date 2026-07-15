#[cfg(feature = "nova")]
use nes_core::NesCore;
#[cfg(feature = "nova")]
use pixels::{Pixels, SurfaceTexture};
#[cfg(feature = "nova")]
use winit::{
    dpi::LogicalSize,
    event::{Event, WindowEvent},
    event_loop::EventLoopWindowTarget,
    window::{Window, WindowBuilder},
};

#[cfg(feature = "nova")]
pub struct PpuVisualizerState {
    pub window: Window,
    pub pixels: Pixels,
}

#[cfg(feature = "nova")]
impl PpuVisualizerState {
    pub fn new(target: &EventLoopWindowTarget<()>) -> Result<Self, Box<dyn std::error::Error>> {
        let size = LogicalSize::new(256.0, 128.0);
        let window = WindowBuilder::new()
            .with_title("PPU Pattern Tables")
            .with_inner_size(size)
            .build(target)?;

        let window_size = window.inner_size();
        let surface_texture = SurfaceTexture::new(window_size.width, window_size.height, &window);
        let pixels = Pixels::new(256, 128, surface_texture)?;

        Ok(Self { window, pixels })
    }

    pub fn handle_event(&mut self, event: &Event<()>) {
        if let Event::WindowEvent {
            window_id,
            event: WindowEvent::Resized(size),
        } = event
        {
            if *window_id == self.window.id() {
                let _ = self.pixels.resize_surface(size.width, size.height);
            }
        }
    }

    pub fn render(&mut self, core: &NesCore) {
        let frame = self.pixels.frame_mut();
        // Left Pattern Table
        let mut left_buffer = vec![0; 128 * 128 * 4];
        nes_core::experimental::ppu_visualizer::PpuVisualizer::render_pattern_table_left(
            core,
            &mut left_buffer,
        );

        // Right Pattern Table
        let mut right_buffer = vec![0; 128 * 128 * 4];
        nes_core::experimental::ppu_visualizer::PpuVisualizer::render_pattern_table_right(
            core,
            &mut right_buffer,
        );

        for y in 0..128 {
            for x in 0..128 {
                let dst_left = (y * 256 + x) * 4;
                let src_left = (y * 128 + x) * 4;
                frame[dst_left..dst_left + 4].copy_from_slice(&left_buffer[src_left..src_left + 4]);

                let dst_right = (y * 256 + 128 + x) * 4;
                let src_right = (y * 128 + x) * 4;
                frame[dst_right..dst_right + 4]
                    .copy_from_slice(&right_buffer[src_right..src_right + 4]);
            }
        }
        let _ = self.pixels.render();
    }
}
