use std::env;
use std::fs;
use std::path::PathBuf;
use std::time::{Duration, Instant};

#[cfg(feature = "mcp-host")]
mod mcp_host;

use nes_config::{
    DEFAULT_CONFIG_PATH, NesConfig, StepModeConfig, normalize_nonzero_u32, normalize_nonzero_u64,
    parse_config_path_arg,
};
use nes_core::{AUDIO_SAMPLE_RATE, Command, FRAME_HEIGHT, FRAME_RGBA_BYTES, FRAME_WIDTH, NesCore};
use nes_desktop::app::map_key_event_to_command;
use pixels::{Pixels, SurfaceTexture};
use rodio::{OutputStream, Sink, buffer::SamplesBuffer};
use winit::dpi::LogicalSize;
use winit::event::{ElementState, Event, VirtualKeyCode, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::window::WindowBuilder;

#[cfg(feature = "mcp-host")]
use crate::mcp_host::McpHost;

const DEFAULT_CPU_STEPS_PER_FRAME: u32 = 10_000;
const DEFAULT_WINDOW_SCALE: u32 = 3;
const TARGET_FRAME_TIME: Duration = Duration::from_micros(16_667);
const MAX_AUDIO_QUEUE_CHUNKS: usize = 8;
const AUDIO_CHANNELS: u16 = 1;
const DEFAULT_METRICS_EVERY_FRAMES: u64 = 60;
const DEFAULT_TRACE_EVERY_FRAMES: u64 = 0;
const DEFAULT_CAPTURE_EVERY_FRAMES: u64 = 1;
const DEFAULT_MCP_BIND_ADDR: &str = "127.0.0.1:6502";

struct RuntimeConfig {
    rom_path: String,
    window_scale: u32,
    step_mode: StepMode,
    audio_enabled: bool,
    trace_every_frames: u64,
    metrics_enabled: bool,
    metrics_every_frames: u64,
    capture: Option<CaptureConfig>,
    loaded_config_path: Option<PathBuf>,
    mcp_enabled: bool,
    mcp_bind_addr: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StepMode {
    CpuBudget(u32),
    Frame,
}

struct CaptureConfig {
    path_template: String,
    every_n_frames: u64,
}

struct AudioOutput {
    sink: Sink,
    _stream: OutputStream,
}

struct PerfMetrics {
    enabled: bool,
    every_n_frames: u64,
    report_start: Instant,
    report_start_ppu_frame: u64,
    report_frames: u64,
    step_work: Duration,
    render_work: Duration,
    late_frames: u64,
    last_pc: Option<u16>,
    pc_stall_frames: u64,
    last_frame_signature: Option<u64>,
    unchanged_frame_count: u64,
    warned_stall: bool,
    audio_queue_peak: usize,
    audio_queue_drops: u64,
}

impl PerfMetrics {
    fn new(enabled: bool, every_n_frames: u64, initial_ppu_frame: u64) -> Self {
        Self {
            enabled,
            every_n_frames: normalize_nonzero_u64(every_n_frames, DEFAULT_METRICS_EVERY_FRAMES),
            report_start: Instant::now(),
            report_start_ppu_frame: initial_ppu_frame,
            report_frames: 0,
            step_work: Duration::ZERO,
            render_work: Duration::ZERO,
            late_frames: 0,
            last_pc: None,
            pc_stall_frames: 0,
            last_frame_signature: None,
            unchanged_frame_count: 0,
            warned_stall: false,
            audio_queue_peak: 0,
            audio_queue_drops: 0,
        }
    }

    fn on_step(&mut self, core: &NesCore, step_elapsed: Duration, missed_deadline: bool) {
        if !self.enabled {
            return;
        }
        self.report_frames = self.report_frames.saturating_add(1);
        self.step_work = self.step_work.saturating_add(step_elapsed);
        if missed_deadline {
            self.late_frames = self.late_frames.saturating_add(1);
        }
        let pc = core.cpu_pc();
        if self.last_pc == Some(pc) {
            self.pc_stall_frames = self.pc_stall_frames.saturating_add(1);
        } else {
            self.pc_stall_frames = 0;
            self.warned_stall = false;
        }
        self.last_pc = Some(pc);
        if self.pc_stall_frames >= 240 && !self.warned_stall {
            let status = core.read_memory(0x2002);
            let sprite0_y = core.ppu_oam_byte(0);
            let sprite0_x = core.ppu_oam_byte(3);
            eprintln!(
                "[warn] long pc stall detected: pc=${:04X} stall_frames={} ppu_frame={} scanline={} dot={} status={:02X} sprite0=({}, {})",
                pc,
                self.pc_stall_frames,
                core.ppu_frame_counter(),
                core.ppu_scanline(),
                core.ppu_dot(),
                status,
                sprite0_x,
                sprite0_y
            );
            self.warned_stall = true;
        }
    }

    fn on_render(&mut self, frame: &[u8], render_elapsed: Duration) {
        if !self.enabled {
            return;
        }
        self.render_work = self.render_work.saturating_add(render_elapsed);
        let signature = frame_signature(frame);
        if self.last_frame_signature == Some(signature) {
            self.unchanged_frame_count = self.unchanged_frame_count.saturating_add(1);
        } else {
            self.unchanged_frame_count = 0;
        }
        self.last_frame_signature = Some(signature);
    }

    fn maybe_report(&mut self, core: &NesCore) {
        if !self.enabled || self.report_frames < self.every_n_frames {
            return;
        }
        let elapsed = self.report_start.elapsed().as_secs_f64();
        if elapsed <= f64::EPSILON {
            return;
        }
        let wall_fps = self.report_frames as f64 / elapsed;
        let ppu_now = core.ppu_frame_counter();
        let emu_fps = ppu_now.saturating_sub(self.report_start_ppu_frame) as f64 / elapsed;
        let avg_step_ms = self.step_work.as_secs_f64() * 1_000.0 / self.report_frames as f64;
        let avg_render_ms = self.render_work.as_secs_f64() * 1_000.0 / self.report_frames as f64;

        println!(
            "[metrics] wall_fps={wall_fps:.1} emu_fps={emu_fps:.1} avg_step_ms={avg_step_ms:.2} avg_render_ms={avg_render_ms:.2} late_frames={} pc_stall_frames={} unchanged_frames={} audio_peak_q={} audio_drop_chunks={}",
            self.late_frames,
            self.pc_stall_frames,
            self.unchanged_frame_count,
            self.audio_queue_peak,
            self.audio_queue_drops
        );

        self.report_start = Instant::now();
        self.report_start_ppu_frame = ppu_now;
        self.report_frames = 0;
        self.step_work = Duration::ZERO;
        self.render_work = Duration::ZERO;
        self.late_frames = 0;
        self.audio_queue_peak = 0;
        self.audio_queue_drops = 0;
    }

    fn on_audio_queue(&mut self, queue_depth: usize, dropped: bool) {
        if !self.enabled {
            return;
        }
        self.audio_queue_peak = self.audio_queue_peak.max(queue_depth);
        if dropped {
            self.audio_queue_drops = self.audio_queue_drops.saturating_add(1);
        }
    }
}

impl AudioOutput {
    fn try_new() -> Result<Self, String> {
        let (stream, handle) = OutputStream::try_default()
            .map_err(|err| format!("Audio output init failed: {err}"))?;
        let sink =
            Sink::try_new(&handle).map_err(|err| format!("Audio sink init failed: {err}"))?;
        Ok(Self {
            sink,
            _stream: stream,
        })
    }

    fn queue_samples(&self, samples: Vec<i16>) -> bool {
        if self.sink.len() >= MAX_AUDIO_QUEUE_CHUNKS {
            return false;
        }
        self.sink.append(SamplesBuffer::new(
            AUDIO_CHANNELS,
            AUDIO_SAMPLE_RATE,
            samples,
        ));
        true
    }

    fn queue_len(&self) -> usize {
        self.sink.len()
    }
}

impl Drop for AudioOutput {
    fn drop(&mut self) {
        // Ensure sink teardown happens while the stream backend is still alive.
        self.sink.stop();
    }
}

fn main() {
    if let Err(err) = run() {
        eprintln!("{err}");
    }
}

fn run() -> Result<(), String> {
    let runtime = resolve_runtime_config()?;

    #[cfg(not(feature = "mcp-host"))]
    if runtime.mcp_enabled {
        return Err(format!(
            "MCP host requested for {} but this build does not include the `mcp-host` feature.",
            runtime.mcp_bind_addr
        ));
    }

    let rom_path = runtime.rom_path.clone();
    let rom_bytes =
        fs::read(&rom_path).map_err(|err| format!("Failed to read ROM at '{rom_path}': {err}"))?;

    let mut core = NesCore::new();
    let info = core
        .load_ines_rom(&rom_bytes)
        .map_err(|err| format!("Failed to load ROM: {err}"))?;
    let step_mode = runtime.step_mode;

    println!("Loaded ROM: {rom_path}");
    println!(
        "Mapper {}, PRG {} bytes, reset vector ${:04X}",
        info.mapper_id, info.prg_rom_bytes, info.reset_pc
    );
    if let Some(config_path) = runtime.loaded_config_path.as_ref() {
        println!("Config: {}", config_path.display());
    }
    println!("Controls: Z=A, X=B, Enter=Start, RightShift=Select, Arrows=D-pad, Esc=Quit");
    match step_mode {
        StepMode::Frame => println!("Step mode: frame"),
        StepMode::CpuBudget(steps) => println!("Step mode: cpu ({steps} instructions/frame)"),
    }
    if cfg!(debug_assertions) {
        eprintln!(
            "Running debug build; performance will be much lower. For speed use: cargo run -p nes-desktop --release -- <rom>"
        );
    }

    #[cfg(feature = "mcp-host")]
    let mcp_host = if runtime.mcp_enabled {
        let host = McpHost::start(&runtime.mcp_bind_addr)?;
        println!("MCP host: tcp://{}", host.bind_addr());
        Some(host)
    } else {
        None
    };

    let event_loop = EventLoop::new();
    let window = WindowBuilder::new()
        .with_title("nes-desktop")
        .with_inner_size(LogicalSize::new(
            f64::from(FRAME_WIDTH as u32 * runtime.window_scale),
            f64::from(FRAME_HEIGHT as u32 * runtime.window_scale),
        ))
        .with_min_inner_size(LogicalSize::new(FRAME_WIDTH as f64, FRAME_HEIGHT as f64))
        .build(&event_loop)
        .map_err(|err| format!("Failed to create window: {err}"))?;

    let window_size = window.inner_size();
    let surface_texture = SurfaceTexture::new(window_size.width, window_size.height, &window);
    let mut pixels = Pixels::new(FRAME_WIDTH as u32, FRAME_HEIGHT as u32, surface_texture)
        .map_err(|err| format!("Failed to create pixel surface: {err}"))?;

    let mut frame_index = 0_u64;
    let mut frame_rgba = vec![0_u8; FRAME_RGBA_BYTES];
    let mut next_frame_deadline = Instant::now();
    let capture = runtime.capture;
    let mut metrics = PerfMetrics::new(
        runtime.metrics_enabled,
        runtime.metrics_every_frames,
        core.ppu_frame_counter(),
    );
    let trace_every_frames = runtime.trace_every_frames;

    let audio_output = if runtime.audio_enabled {
        match AudioOutput::try_new() {
            Ok(output) => Some(output),
            Err(err) => {
                eprintln!("{err}");
                eprintln!("Continuing without audio output.");
                None
            }
        }
    } else {
        eprintln!("Audio disabled by config.");
        None
    };

    event_loop.run(move |event, _, control_flow| match event {
        Event::WindowEvent { event, .. } => match event {
            WindowEvent::CloseRequested => {
                *control_flow = ControlFlow::Exit;
            }
            WindowEvent::KeyboardInput { input, .. } => {
                let Some(key) = input.virtual_keycode else {
                    return;
                };
                let pressed = input.state == ElementState::Pressed;

                if key == VirtualKeyCode::Escape && pressed {
                    *control_flow = ControlFlow::Exit;
                    return;
                }

                if let Some(key_code) = map_virtual_keycode(key)
                    && let Some(mapped) = map_key_event_to_command(key_code, pressed)
                    && let Err(err) = core.execute(mapped.core)
                {
                    eprintln!("Input command failed: {err}");
                    *control_flow = ControlFlow::Exit;
                }
            }
            WindowEvent::Resized(size) => {
                if let Err(err) = pixels.resize_surface(size.width, size.height) {
                    eprintln!("Surface resize failed: {err}");
                    *control_flow = ControlFlow::Exit;
                }
            }
            WindowEvent::ScaleFactorChanged { new_inner_size, .. } => {
                if let Err(err) = pixels.resize_surface(new_inner_size.width, new_inner_size.height)
                {
                    eprintln!("Scale-factor resize failed: {err}");
                    *control_flow = ControlFlow::Exit;
                }
            }
            _ => {}
        },
        Event::MainEventsCleared => {
            #[cfg(feature = "mcp-host")]
            if let Some(host) = mcp_host.as_ref() {
                host.drain(&mut core);
            }

            let now = Instant::now();
            if now < next_frame_deadline {
                *control_flow = ControlFlow::WaitUntil(next_frame_deadline);
                return;
            }
            let missed_deadline = now > next_frame_deadline;
            next_frame_deadline = now + TARGET_FRAME_TIME;
            let step_start = Instant::now();

            if let Err(err) = advance_core_for_host_frame(&mut core, step_mode) {
                eprintln!("CPU halted at PC ${:04X}: {err}", core.cpu_pc());
                *control_flow = ControlFlow::Exit;
                return;
            }

            let step_elapsed = step_start.elapsed();
            frame_index = frame_index.saturating_add(1);
            metrics.on_step(&core, step_elapsed, missed_deadline);
            if trace_every_frames > 0 && frame_index.is_multiple_of(trace_every_frames) {
                let regs = core.cpu_snapshot();
                println!(
                    "frame={} ppu_frame={} pc=${:04X} a={:02X} x={:02X} y={:02X} ctrl={:02X}",
                    frame_index,
                    core.ppu_frame_counter(),
                    regs.pc,
                    regs.a,
                    regs.x,
                    regs.y,
                    core.controller_bits()
                );
            }

            if let Some(audio_output) = audio_output.as_ref() {
                let queued = audio_output.queue_samples(core.audio_chunk_i16());
                metrics.on_audio_queue(audio_output.queue_len(), !queued);
            }

            window.request_redraw();
            *control_flow = ControlFlow::WaitUntil(next_frame_deadline);
        }
        Event::RedrawRequested(_) => {
            let render_start = Instant::now();
            core.fill_framebuffer_rgba(&mut frame_rgba);
            pixels.frame_mut().copy_from_slice(&frame_rgba);
            if let Some(config) = capture.as_ref()
                && config.every_n_frames != 0
                && frame_index.is_multiple_of(config.every_n_frames)
            {
                let path = capture_path_for_frame(&config.path_template, frame_index);
                if let Err(err) = write_frame_ppm(&path, &frame_rgba) {
                    eprintln!("Frame capture failed at frame {frame_index}: {err}");
                }
            }

            if let Err(err) = pixels.render() {
                eprintln!("Render failed: {err}");
                *control_flow = ControlFlow::Exit;
                return;
            }
            metrics.on_render(&frame_rgba, render_start.elapsed());
            metrics.maybe_report(&core);
        }
        _ => {
            *control_flow = ControlFlow::WaitUntil(next_frame_deadline);
        }
    });
}

fn resolve_runtime_config() -> Result<RuntimeConfig, String> {
    let raw_args: Vec<String> = env::args().skip(1).collect();
    let (config_path, pass_through) = parse_config_path_arg(&raw_args)?;
    let (rom_path_arg, mcp_enabled, mcp_bind_addr) = parse_runtime_args(&pass_through)?;

    let loaded_config_path = config_path.clone().or_else(|| {
        let default_path = PathBuf::from(DEFAULT_CONFIG_PATH);
        if default_path.exists() {
            Some(default_path)
        } else {
            None
        }
    });
    let config = NesConfig::load_or_default(config_path.as_deref())?;

    let rom_path = rom_path_arg
        .or_else(|| config.desktop.rom_path.clone())
        .or_else(|| config.roms.smb.clone())
        .ok_or_else(|| {
            format!(
                "ROM path not configured. Provide a positional ROM argument or set `desktop.rom_path`/`roms.smb` in {DEFAULT_CONFIG_PATH}."
            )
        })?;
    let window_scale = normalize_nonzero_u32(config.desktop.window_scale, DEFAULT_WINDOW_SCALE);
    let cpu_steps_per_frame = normalize_nonzero_u32(
        config.desktop.cpu_steps_per_frame,
        DEFAULT_CPU_STEPS_PER_FRAME,
    );
    let trace_every_frames = normalize_nonzero_u64(
        config.desktop.trace_every_frames,
        DEFAULT_TRACE_EVERY_FRAMES,
    );
    let metrics_every_frames = normalize_nonzero_u64(
        config.desktop.metrics_every_frames,
        DEFAULT_METRICS_EVERY_FRAMES,
    );
    let capture = capture_config_from_parts(
        config.desktop.capture_path_template,
        config.desktop.capture_every_frames,
    );
    let step_mode = match config.desktop.step_mode {
        StepModeConfig::Frame => StepMode::Frame,
        StepModeConfig::Cpu => StepMode::CpuBudget(cpu_steps_per_frame),
    };

    Ok(RuntimeConfig {
        rom_path,
        window_scale,
        step_mode,
        audio_enabled: config.desktop.audio_enabled,
        trace_every_frames,
        metrics_enabled: config.desktop.metrics_enabled,
        metrics_every_frames,
        capture,
        loaded_config_path,
        mcp_enabled,
        mcp_bind_addr,
    })
}

fn parse_runtime_args(args: &[String]) -> Result<(Option<String>, bool, String), String> {
    let mut rom_path_arg = None::<String>;
    let mut mcp_enabled = false;
    let mut mcp_bind_addr = DEFAULT_MCP_BIND_ADDR.to_owned();
    let mut idx = 0_usize;
    while idx < args.len() {
        let arg = &args[idx];
        if arg == "--help" || arg == "-h" {
            return Err(format!(
                "Usage: nes-desktop [--config <path>] [--mcp-host] [--mcp-bind <addr>] [rom_path]\nDefault config path: {DEFAULT_CONFIG_PATH}"
            ));
        }
        if arg == "--mcp-host" {
            mcp_enabled = true;
            idx += 1;
            continue;
        }
        if arg == "--mcp-bind" {
            let Some(bind_addr) = args.get(idx + 1) else {
                return Err("missing value after --mcp-bind".to_owned());
            };
            mcp_bind_addr = bind_addr.clone();
            idx += 2;
            continue;
        }
        if let Some(bind_addr) = arg.strip_prefix("--mcp-bind=") {
            if bind_addr.is_empty() {
                return Err("missing value after --mcp-bind=".to_owned());
            }
            mcp_bind_addr = bind_addr.to_owned();
            idx += 1;
            continue;
        }
        if arg.starts_with("--") {
            return Err(format!(
                "unknown flag '{arg}'. Usage: nes-desktop [--config <path>] [--mcp-host] [--mcp-bind <addr>] [rom_path]"
            ));
        }
        if rom_path_arg.is_some() {
            return Err(
                "multiple ROM paths provided; expected at most one positional ROM path".to_owned(),
            );
        }
        rom_path_arg = Some(arg.clone());
        idx += 1;
    }
    Ok((rom_path_arg, mcp_enabled, mcp_bind_addr))
}

fn map_virtual_keycode(key: VirtualKeyCode) -> Option<&'static str> {
    match key {
        VirtualKeyCode::Z => Some("KeyZ"),
        VirtualKeyCode::X => Some("KeyX"),
        VirtualKeyCode::Return => Some("Enter"),
        VirtualKeyCode::RShift => Some("ShiftRight"),
        VirtualKeyCode::Up => Some("ArrowUp"),
        VirtualKeyCode::Down => Some("ArrowDown"),
        VirtualKeyCode::Left => Some("ArrowLeft"),
        VirtualKeyCode::Right => Some("ArrowRight"),
        _ => None,
    }
}

fn advance_core_for_host_frame(core: &mut NesCore, step_mode: StepMode) -> Result<(), String> {
    match step_mode {
        StepMode::Frame => core
            .execute(Command::StepFrame)
            .map_err(|err| err.to_string()),
        StepMode::CpuBudget(steps) => {
            for _ in 0..steps {
                core.execute(Command::StepCpu)
                    .map_err(|err| err.to_string())?;
            }
            Ok(())
        }
    }
}

fn frame_signature(rgba: &[u8]) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325_u64;
    for idx in (0..rgba.len()).step_by(64) {
        hash ^= u64::from(rgba[idx]);
        hash = hash.wrapping_mul(0x0000_0001_0000_01b3);
    }
    hash
}

fn capture_config_from_parts(
    path_template: Option<String>,
    every_n_frames: u64,
) -> Option<CaptureConfig> {
    let template = path_template?;
    if template.trim().is_empty() {
        return None;
    }
    Some(CaptureConfig {
        path_template: template,
        every_n_frames: normalize_nonzero_u64(every_n_frames, DEFAULT_CAPTURE_EVERY_FRAMES),
    })
}

fn capture_path_for_frame(template: &str, frame: u64) -> String {
    if template.contains("{frame}") {
        template.replace("{frame}", &format!("{frame:06}"))
    } else {
        template.to_owned()
    }
}

fn write_frame_ppm(path: &str, rgba: &[u8]) -> Result<(), String> {
    if rgba.len() != FRAME_RGBA_BYTES {
        return Err("frame length mismatch".to_owned());
    }
    let bytes = if path.to_ascii_lowercase().ends_with(".bmp") {
        encode_bmp(FRAME_WIDTH, FRAME_HEIGHT, rgba)?
    } else {
        encode_ppm(FRAME_WIDTH, FRAME_HEIGHT, rgba)
    };
    fs::write(path, bytes).map_err(|err| format!("unable to write '{path}': {err}"))
}

fn encode_ppm(width: usize, height: usize, rgba: &[u8]) -> Vec<u8> {
    let mut ppm = Vec::with_capacity(32 + width * height * 3);
    ppm.extend_from_slice(format!("P6\n{width} {height}\n255\n").as_bytes());
    for px in rgba.chunks_exact(4) {
        ppm.extend_from_slice(&px[..3]);
    }
    ppm
}

fn encode_bmp(width: usize, height: usize, rgba: &[u8]) -> Result<Vec<u8>, String> {
    let row_bytes = width
        .checked_mul(3)
        .ok_or_else(|| "bmp row size overflow".to_owned())?;
    let row_padding = (4 - (row_bytes % 4)) % 4;
    let stride = row_bytes
        .checked_add(row_padding)
        .ok_or_else(|| "bmp stride overflow".to_owned())?;
    let pixel_data_size = stride
        .checked_mul(height)
        .ok_or_else(|| "bmp pixel data size overflow".to_owned())?;
    let file_size = 54usize
        .checked_add(pixel_data_size)
        .ok_or_else(|| "bmp file size overflow".to_owned())?;

    let width_i32 = i32::try_from(width).map_err(|_| "bmp width out of range".to_owned())?;
    let height_i32 = i32::try_from(height).map_err(|_| "bmp height out of range".to_owned())?;
    let file_size_u32 =
        u32::try_from(file_size).map_err(|_| "bmp file size out of range".to_owned())?;
    let pixel_data_size_u32 = u32::try_from(pixel_data_size)
        .map_err(|_| "bmp pixel data size out of range".to_owned())?;

    let mut bmp = Vec::with_capacity(file_size);
    bmp.extend_from_slice(b"BM");
    bmp.extend_from_slice(&file_size_u32.to_le_bytes());
    bmp.extend_from_slice(&0u16.to_le_bytes());
    bmp.extend_from_slice(&0u16.to_le_bytes());
    bmp.extend_from_slice(&54u32.to_le_bytes());

    bmp.extend_from_slice(&40u32.to_le_bytes());
    bmp.extend_from_slice(&width_i32.to_le_bytes());
    bmp.extend_from_slice(&height_i32.to_le_bytes());
    bmp.extend_from_slice(&1u16.to_le_bytes());
    bmp.extend_from_slice(&24u16.to_le_bytes());
    bmp.extend_from_slice(&0u32.to_le_bytes());
    bmp.extend_from_slice(&pixel_data_size_u32.to_le_bytes());
    bmp.extend_from_slice(&2_835u32.to_le_bytes());
    bmp.extend_from_slice(&2_835u32.to_le_bytes());
    bmp.extend_from_slice(&0u32.to_le_bytes());
    bmp.extend_from_slice(&0u32.to_le_bytes());

    for y in (0..height).rev() {
        let row_start = y * width * 4;
        for x in 0..width {
            let idx = row_start + x * 4;
            bmp.push(rgba[idx + 2]);
            bmp.push(rgba[idx + 1]);
            bmp.push(rgba[idx]);
        }
        bmp.extend(std::iter::repeat_n(0, row_padding));
    }
    Ok(bmp)
}

#[cfg(test)]
mod tests {
    use super::{DEFAULT_MCP_BIND_ADDR, parse_runtime_args};

    #[test]
    fn parse_runtime_args_accepts_mcp_host_and_bind_flags() {
        let args = vec![
            "--mcp-host".to_owned(),
            "--mcp-bind".to_owned(),
            "127.0.0.1:7777".to_owned(),
            "game.nes".to_owned(),
        ];
        let (rom_path, mcp_enabled, bind_addr) = parse_runtime_args(&args).expect("parse args");
        assert_eq!(rom_path.as_deref(), Some("game.nes"));
        assert!(mcp_enabled);
        assert_eq!(bind_addr, "127.0.0.1:7777");
    }

    #[test]
    fn parse_runtime_args_accepts_equals_bind_form() {
        let args = vec!["--mcp-bind=127.0.0.1:7000".to_owned()];
        let (rom_path, mcp_enabled, bind_addr) = parse_runtime_args(&args).expect("parse args");
        assert!(rom_path.is_none());
        assert!(!mcp_enabled);
        assert_eq!(bind_addr, "127.0.0.1:7000");
    }

    #[test]
    fn parse_runtime_args_defaults_bind_when_flag_absent() {
        let args = vec!["game.nes".to_owned()];
        let (rom_path, mcp_enabled, bind_addr) = parse_runtime_args(&args).expect("parse args");
        assert_eq!(rom_path.as_deref(), Some("game.nes"));
        assert!(!mcp_enabled);
        assert_eq!(bind_addr, DEFAULT_MCP_BIND_ADDR);
    }

    #[test]
    fn parse_runtime_args_rejects_unknown_flags() {
        let args = vec!["--bogus".to_owned()];
        let err = parse_runtime_args(&args).expect_err("unknown flag should fail");
        assert!(err.contains("unknown flag"));
    }
}
