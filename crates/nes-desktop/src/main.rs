use std::env;
use std::fs;
use std::time::{Duration, Instant};

use nes_core::{AUDIO_SAMPLE_RATE, Command, FRAME_HEIGHT, FRAME_RGBA_BYTES, FRAME_WIDTH, NesCore};
use nes_desktop::app::map_key_event_to_command;
use pixels::{Pixels, SurfaceTexture};
use rodio::{OutputStream, Sink, buffer::SamplesBuffer};
use winit::dpi::LogicalSize;
use winit::event::{ElementState, Event, VirtualKeyCode, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::window::WindowBuilder;

const DEFAULT_CPU_STEPS_PER_FRAME: u32 = 10_000;
const DEFAULT_WINDOW_SCALE: u32 = 3;
const TARGET_FRAME_TIME: Duration = Duration::from_micros(16_667);
const MAX_AUDIO_QUEUE_CHUNKS: usize = 4;
const AUDIO_CHANNELS: u16 = 1;

struct AudioOutput {
    _stream: OutputStream,
    sink: Sink,
}

impl AudioOutput {
    fn try_new() -> Result<Self, String> {
        let (stream, handle) = OutputStream::try_default()
            .map_err(|err| format!("Audio output init failed: {err}"))?;
        let sink =
            Sink::try_new(&handle).map_err(|err| format!("Audio sink init failed: {err}"))?;
        Ok(Self {
            _stream: stream,
            sink,
        })
    }

    fn queue_samples(&self, samples: Vec<i16>) {
        if self.sink.len() >= MAX_AUDIO_QUEUE_CHUNKS {
            return;
        }
        self.sink.append(SamplesBuffer::new(
            AUDIO_CHANNELS,
            AUDIO_SAMPLE_RATE,
            samples,
        ));
    }
}

fn main() {
    if let Err(err) = run() {
        eprintln!("{err}");
    }
}

fn run() -> Result<(), String> {
    let rom_path = resolve_rom_path()?;
    let rom_bytes =
        fs::read(&rom_path).map_err(|err| format!("Failed to read ROM at '{rom_path}': {err}"))?;

    let mut core = NesCore::new();
    let info = core
        .load_ines_rom(&rom_bytes)
        .map_err(|err| format!("Failed to load ROM: {err}"))?;

    let steps_per_frame = env::var("NES_CPU_STEPS_PER_FRAME")
        .ok()
        .and_then(|v| v.parse::<u32>().ok())
        .filter(|v| *v > 0)
        .unwrap_or(DEFAULT_CPU_STEPS_PER_FRAME);

    let window_scale = env::var("NES_WINDOW_SCALE")
        .ok()
        .and_then(|v| v.parse::<u32>().ok())
        .filter(|v| *v > 0)
        .unwrap_or(DEFAULT_WINDOW_SCALE);

    println!("Loaded ROM: {rom_path}");
    println!(
        "Mapper {}, PRG {} bytes, reset vector ${:04X}",
        info.mapper_id, info.prg_rom_bytes, info.reset_pc
    );
    println!("Controls: Z=A, X=B, Enter=Start, RightShift=Select, Arrows=D-pad, Esc=Quit");

    let event_loop = EventLoop::new();
    let window = WindowBuilder::new()
        .with_title("nes-desktop")
        .with_inner_size(LogicalSize::new(
            f64::from(FRAME_WIDTH as u32 * window_scale),
            f64::from(FRAME_HEIGHT as u32 * window_scale),
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

    let audio_output = match AudioOutput::try_new() {
        Ok(output) => Some(output),
        Err(err) => {
            eprintln!("{err}");
            eprintln!("Continuing without audio output.");
            None
        }
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
            let now = Instant::now();
            if now < next_frame_deadline {
                *control_flow = ControlFlow::WaitUntil(next_frame_deadline);
                return;
            }
            next_frame_deadline = now + TARGET_FRAME_TIME;

            let mut cpu_halted = false;
            for _ in 0..steps_per_frame {
                if let Err(err) = core.execute(Command::StepCpu) {
                    eprintln!("CPU halted at PC ${:04X}: {err}", core.cpu_pc());
                    *control_flow = ControlFlow::Exit;
                    cpu_halted = true;
                    break;
                }
            }
            if cpu_halted {
                return;
            }

            frame_index = frame_index.saturating_add(1);
            if frame_index.is_multiple_of(30) {
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
                audio_output.queue_samples(core.audio_chunk_i16());
            }

            window.request_redraw();
            *control_flow = ControlFlow::WaitUntil(next_frame_deadline);
        }
        Event::RedrawRequested(_) => {
            core.fill_framebuffer_rgba(&mut frame_rgba);
            pixels.frame_mut().copy_from_slice(&frame_rgba);

            if let Err(err) = pixels.render() {
                eprintln!("Render failed: {err}");
                *control_flow = ControlFlow::Exit;
            }
        }
        _ => {
            *control_flow = ControlFlow::WaitUntil(next_frame_deadline);
        }
    });
}

fn resolve_rom_path() -> Result<String, String> {
    if let Some(path) = env::args().nth(1) {
        return Ok(path);
    }
    if let Ok(path) = env::var("SMB_ROM_PATH")
        && !path.trim().is_empty()
    {
        return Ok(path);
    }
    Err("Provide ROM path as first argument or set SMB_ROM_PATH.".to_owned())
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
