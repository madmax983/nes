use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::path::PathBuf;
use std::time::{Duration, Instant};

#[cfg(feature = "mcp-host")]
mod mcp_host;
mod netplay;

use gilrs::{Axis as GamepadAxis, Button as GamepadButton, GamepadId, Gilrs};
use nes_config::{
    DEFAULT_CONFIG_PATH, NesConfig, StepModeConfig, normalize_nonzero_u32, normalize_nonzero_u64,
    parse_config_path_arg,
};
use nes_core::{
    AUDIO_SAMPLE_RATE, Button, Command, FRAME_HEIGHT, FRAME_RGBA_BYTES, FRAME_WIDTH, NesCore,
};
use nes_desktop::app::{map_key_event_to_button_bit, map_key_event_to_command};
use nes_netplay::{HashComparison, RollbackConfig, RollbackEngine, ServerMessage};
use pixels::{Pixels, SurfaceTexture};
use rodio::{OutputStream, Sink, buffer::SamplesBuffer};
use winit::dpi::LogicalSize;
use winit::event::{ElementState, Event, VirtualKeyCode, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::window::WindowBuilder;

#[cfg(feature = "mcp-host")]
use crate::mcp_host::McpHost;
use crate::netplay::{NetplayClient, NetplayRuntimeConfig};

const DEFAULT_CPU_STEPS_PER_FRAME: u32 = 10_000;
const DEFAULT_WINDOW_SCALE: u32 = 3;
const TARGET_FRAME_TIME: Duration = Duration::from_micros(16_667);
const MAX_AUDIO_QUEUE_CHUNKS: usize = 8;
const AUDIO_CHANNELS: u16 = 1;
const DEFAULT_METRICS_EVERY_FRAMES: u64 = 60;
const DEFAULT_TRACE_EVERY_FRAMES: u64 = 0;
const DEFAULT_CAPTURE_EVERY_FRAMES: u64 = 1;
const DEFAULT_MCP_BIND_ADDR: &str = "127.0.0.1:6502";
const GAMEPAD_AXIS_THRESHOLD: f32 = 0.5;
const NETPLAY_PING_INTERVAL: Duration = Duration::from_millis(500);
const NETPLAY_AUTO_DELAY_MIN_FRAMES: u32 = 1;
const NETPLAY_AUTO_DELAY_MAX_FRAMES: u32 = 12;
const CONTROLLER_BUTTONS: [Button; 8] = [
    Button::A,
    Button::B,
    Button::Select,
    Button::Start,
    Button::Up,
    Button::Down,
    Button::Left,
    Button::Right,
];

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
    netplay: Option<NetplayRuntimeConfig>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct RuntimeArgs {
    rom_path: Option<String>,
    mcp_enabled: bool,
    mcp_bind_addr: String,
    netplay_enabled: bool,
    netplay_relay_addr: Option<String>,
    netplay_room: Option<String>,
    netplay_player: Option<u8>,
    netplay_input_delay_frames: Option<u32>,
    netplay_max_rollback_frames: Option<u32>,
    netplay_hash_check_every_frames: Option<u64>,
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

struct NetplayRuntimeStats {
    latest_rtt_ms: Option<f64>,
    jitter_ms: f64,
    rollback_count: u64,
    max_rollback_distance: u64,
    desync_count: u64,
    input_delay_frames: u32,
}

impl NetplayRuntimeStats {
    fn new(input_delay_frames: u32) -> Self {
        Self {
            latest_rtt_ms: None,
            jitter_ms: 0.0,
            rollback_count: 0,
            max_rollback_distance: 0,
            desync_count: 0,
            input_delay_frames,
        }
    }

    fn observe_rtt_ms(&mut self, rtt_ms: f64) {
        if let Some(previous) = self.latest_rtt_ms {
            let delta = (rtt_ms - previous).abs();
            if self.jitter_ms <= f64::EPSILON {
                self.jitter_ms = delta;
            } else {
                // RFC3550-style EWMA jitter estimator.
                self.jitter_ms += (delta - self.jitter_ms) * 0.125;
            }
        }
        self.latest_rtt_ms = Some(rtt_ms);
    }

    fn observe_rollback(&mut self, distance: u64) {
        if distance == 0 {
            return;
        }
        self.rollback_count = self.rollback_count.saturating_add(1);
        self.max_rollback_distance = self.max_rollback_distance.max(distance);
    }

    fn observe_desync(&mut self) {
        self.desync_count = self.desync_count.saturating_add(1);
    }

    fn latest_rtt_ms_or_zero(&self) -> f64 {
        self.latest_rtt_ms.unwrap_or(0.0)
    }
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
    netplay_rtt_ms: f64,
    netplay_jitter_ms: f64,
    netplay_rollbacks: u64,
    netplay_max_rollback_distance: u64,
    netplay_desyncs: u64,
    netplay_input_delay_frames: u32,
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
            netplay_rtt_ms: 0.0,
            netplay_jitter_ms: 0.0,
            netplay_rollbacks: 0,
            netplay_max_rollback_distance: 0,
            netplay_desyncs: 0,
            netplay_input_delay_frames: 0,
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
            "[metrics] wall_fps={wall_fps:.1} emu_fps={emu_fps:.1} avg_step_ms={avg_step_ms:.2} avg_render_ms={avg_render_ms:.2} late_frames={} pc_stall_frames={} unchanged_frames={} audio_peak_q={} audio_drop_chunks={} net_rtt_ms={:.1} net_jitter_ms={:.1} net_rollbacks={} net_max_rb={} net_desyncs={} net_delay_frames={}",
            self.late_frames,
            self.pc_stall_frames,
            self.unchanged_frame_count,
            self.audio_queue_peak,
            self.audio_queue_drops,
            self.netplay_rtt_ms,
            self.netplay_jitter_ms,
            self.netplay_rollbacks,
            self.netplay_max_rollback_distance,
            self.netplay_desyncs,
            self.netplay_input_delay_frames
        );

        self.report_start = Instant::now();
        self.report_start_ppu_frame = ppu_now;
        self.report_frames = 0;
        self.step_work = Duration::ZERO;
        self.render_work = Duration::ZERO;
        self.late_frames = 0;
        self.audio_queue_peak = 0;
        self.audio_queue_drops = 0;
        self.netplay_rollbacks = 0;
        self.netplay_max_rollback_distance = 0;
        self.netplay_desyncs = 0;
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

    fn on_netplay_stats(&mut self, stats: &NetplayRuntimeStats) {
        if !self.enabled {
            return;
        }
        self.netplay_rtt_ms = stats.latest_rtt_ms_or_zero();
        self.netplay_jitter_ms = stats.jitter_ms;
        self.netplay_rollbacks = stats.rollback_count;
        self.netplay_max_rollback_distance = stats.max_rollback_distance;
        self.netplay_desyncs = stats.desync_count;
        self.netplay_input_delay_frames = stats.input_delay_frames;
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
    println!("Controls: keyboard Z=A, X=B, Enter=Start, RightShift=Select, Arrows=D-pad, Esc=Quit");
    println!("Gamepad: face buttons=A/B, Start/Select, D-pad or left stick");
    match step_mode {
        StepMode::Frame => println!("Step mode: frame"),
        StepMode::CpuBudget(steps) => println!("Step mode: cpu ({steps} instructions/frame)"),
    }
    if let Some(netplay) = runtime.netplay.as_ref() {
        println!(
            "Netplay: relay={} room='{}' player={} delay={} rollback={} hash_every={}",
            netplay.relay_addr,
            netplay.room,
            netplay.player,
            netplay.input_delay_frames,
            netplay.max_rollback_frames,
            netplay.hash_check_every_frames
        );
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

    let netplay_client = if let Some(netplay) = runtime.netplay.as_ref() {
        Some(NetplayClient::connect(netplay)?)
    } else {
        None
    };
    let mut rollback = if let Some(netplay) = runtime.netplay.as_ref() {
        Some(
            RollbackEngine::new(RollbackConfig {
                local_player: netplay.player,
                input_delay_frames: netplay.input_delay_frames,
                max_rollback_frames: netplay.max_rollback_frames,
            })
            .map_err(|err| format!("failed to initialize rollback engine: {err}"))?,
        )
    } else {
        None
    };
    let netplay_hash_check_every = runtime
        .netplay
        .as_ref()
        .map_or(0, |netplay| netplay.hash_check_every_frames);
    let netplay_local_player = runtime.netplay.as_ref().map_or(1, |netplay| netplay.player);
    let mut netplay_stats = runtime
        .netplay
        .as_ref()
        .map(|netplay| NetplayRuntimeStats::new(netplay.input_delay_frames));
    let mut netplay_next_ping_at = Instant::now();
    let mut netplay_ping_nonce = 1_u64;
    let mut netplay_pending_pings = BTreeMap::<u64, Instant>::new();

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
    let mut gilrs = match Gilrs::new() {
        Ok(mut state) => {
            while state.next_event().is_some() {}
            Some(state)
        }
        Err(err) => {
            eprintln!("Gamepad support unavailable: {err}");
            None
        }
    };
    let mut active_gamepads = [None::<GamepadId>; 2];
    if let Some(gilrs_state) = gilrs.as_ref() {
        let connected = connected_gamepad_ids(gilrs_state);
        for (player, slot) in active_gamepads.iter_mut().enumerate() {
            *slot = connected.get(player).copied();
            if let Some(gamepad_id) = *slot {
                println!(
                    "Gamepad P{} connected: {}",
                    player + 1,
                    gilrs_state.gamepad(gamepad_id).name()
                );
            } else {
                println!("Gamepad P{} connected: none", player + 1);
            }
        }
    }
    let mut gamepad_bits = [0_u8; 2];
    let mut keyboard_bits = 0_u8;

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

                if let Some(key_code) = map_virtual_keycode(key) {
                    if rollback.is_some() {
                        if let Some(button_mask) = map_key_event_to_button_bit(key_code) {
                            if pressed {
                                keyboard_bits |= button_mask;
                            } else {
                                keyboard_bits &= !button_mask;
                            }
                        }
                    } else if let Some(mapped) = map_key_event_to_command(key_code, pressed)
                        && let Err(err) = core.execute(mapped.core)
                    {
                        eprintln!("Input command failed: {err}");
                        *control_flow = ControlFlow::Exit;
                    }
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

            if let Some(gilrs_state) = gilrs.as_mut() {
                while gilrs_state.next_event().is_some() {}
                let next_active = select_active_gamepad_ids(gilrs_state, active_gamepads);
                if next_active != active_gamepads {
                    for player in 0..active_gamepads.len() {
                        if next_active[player] != active_gamepads[player] {
                            if let Some(gamepad_id) = next_active[player] {
                                println!(
                                    "Gamepad P{} active: {}",
                                    player + 1,
                                    gilrs_state.gamepad(gamepad_id).name()
                                );
                            } else if active_gamepads[player].is_some() {
                                println!("Gamepad P{} disconnected", player + 1);
                            }
                        }
                    }
                    active_gamepads = next_active;
                }

                for player in 0..gamepad_bits.len() {
                    let next_gamepad_bits = active_gamepads[player]
                        .map(|gamepad_id| sample_gamepad_bits(gilrs_state, gamepad_id))
                        .unwrap_or_default();
                    if rollback.is_none() {
                        for command in controller_state_delta_for_player(
                            gamepad_bits[player],
                            next_gamepad_bits,
                            player == 1,
                        ) {
                            if let Err(err) = core.execute(command) {
                                eprintln!("Gamepad command failed: {err}");
                                *control_flow = ControlFlow::Exit;
                                return;
                            }
                        }
                    }
                    gamepad_bits[player] = next_gamepad_bits;
                }
            }

            let now = Instant::now();
            if now < next_frame_deadline {
                *control_flow = ControlFlow::WaitUntil(next_frame_deadline);
                return;
            }
            let missed_deadline = now > next_frame_deadline;
            next_frame_deadline = now + TARGET_FRAME_TIME;
            let step_start = Instant::now();

            if let Some(rollback_engine) = rollback.as_mut() {
                let local_slot = usize::from(netplay_local_player.saturating_sub(1));
                let local_gamepad_bits = gamepad_bits
                    .get(local_slot)
                    .copied()
                    .unwrap_or_else(|| gamepad_bits.iter().copied().find(|bits| *bits != 0).unwrap_or(0));
                let scheduled = rollback_engine.schedule_local_input(keyboard_bits | local_gamepad_bits);
                if let Some(client) = netplay_client.as_ref()
                    && let Err(err) = client.send_input(scheduled.frame, scheduled.bits)
                {
                    eprintln!("Netplay send input failed: {err}");
                    *control_flow = ControlFlow::Exit;
                    return;
                }
                if let Some(client) = netplay_client.as_ref() {
                    if now >= netplay_next_ping_at {
                        let nonce = netplay_ping_nonce;
                        netplay_ping_nonce = netplay_ping_nonce.wrapping_add(1);
                        netplay_pending_pings.insert(nonce, now);
                        // Bound growth in case a peer vanishes without replying.
                        while netplay_pending_pings.len() > 128 {
                            if let Some(oldest_nonce) =
                                netplay_pending_pings.keys().next().copied()
                            {
                                netplay_pending_pings.remove(&oldest_nonce);
                            }
                        }
                        if let Err(err) = client.send_ping(nonce) {
                            eprintln!("Netplay send ping failed: {err}");
                            *control_flow = ControlFlow::Exit;
                            return;
                        }
                        netplay_next_ping_at = now + NETPLAY_PING_INTERVAL;
                    }

                    loop {
                        let message = match client.try_recv() {
                            Ok(next) => next,
                            Err(err) => {
                                eprintln!("Netplay receive failed: {err}");
                                *control_flow = ControlFlow::Exit;
                                return;
                            }
                        };
                        let Some(message) = message else {
                            break;
                        };
                        match message {
                            ServerMessage::PeerInput { player, frame, bits } => {
                                if player != netplay_local_player {
                                    let ingest = rollback_engine.ingest_remote_input(frame, bits);
                                    if ingest.rollback_queued {
                                        eprintln!(
                                            "[netplay] queued rollback from frame {} due to late remote input",
                                            frame
                                        );
                                    }
                                }
                            }
                            ServerMessage::PeerHash {
                                player,
                                frame,
                                state_hash,
                            } => {
                                if player != netplay_local_player {
                                    match rollback_engine.compare_remote_hash(frame, state_hash) {
                                        HashComparison::Match => {}
                                        HashComparison::Mismatch => {
                                            eprintln!(
                                                "[netplay] desync detected at frame {} (remote hash {:016X})",
                                                frame,
                                                state_hash
                                            );
                                            if let Some(stats) = netplay_stats.as_mut() {
                                                stats.observe_desync();
                                            }
                                        }
                                        HashComparison::PendingLocalFrame => {}
                                    }
                                }
                            }
                            ServerMessage::Joined {
                                room,
                                player,
                                peer_present,
                            } => {
                                println!(
                                    "[netplay] joined room '{}' as P{} (peer_present={})",
                                    room, player, peer_present
                                );
                            }
                            ServerMessage::PeerJoined { player } => {
                                println!("[netplay] peer joined as P{}", player);
                            }
                            ServerMessage::PeerLeft { player } => {
                                println!("[netplay] peer left (P{})", player);
                            }
                            ServerMessage::Error { message } => {
                                eprintln!("[netplay] relay error: {message}");
                                *control_flow = ControlFlow::Exit;
                                return;
                            }
                            ServerMessage::Pong { nonce } => {
                                if let Some(sent_at) = netplay_pending_pings.remove(&nonce) {
                                    let rtt_ms = sent_at.elapsed().as_secs_f64() * 1_000.0;
                                    if let Some(stats) = netplay_stats.as_mut() {
                                        stats.observe_rtt_ms(rtt_ms);
                                    }
                                }
                            }
                        }
                    }
                }

                match rollback_engine.advance_frame(&mut core) {
                    Ok(step) => {
                        if step.rollback_distance > 0 {
                            eprintln!(
                                "[netplay] rollback={} frame={} local={:02X} remote={:02X}",
                                step.rollback_distance, step.frame, step.local_bits, step.remote_bits
                            );
                            if let Some(stats) = netplay_stats.as_mut() {
                                stats.observe_rollback(step.rollback_distance);
                            }
                        }

                        let current_delay = rollback_engine.input_delay_frames();
                        let max_auto_delay = rollback_engine.max_rollback_frames().clamp(
                            NETPLAY_AUTO_DELAY_MIN_FRAMES,
                            NETPLAY_AUTO_DELAY_MAX_FRAMES,
                        );
                        let target_delay = if let Some(stats) = netplay_stats.as_ref() {
                            recommended_input_delay_frames(
                                stats.latest_rtt_ms,
                                stats.jitter_ms,
                                NETPLAY_AUTO_DELAY_MIN_FRAMES,
                                max_auto_delay,
                                current_delay,
                            )
                        } else {
                            current_delay
                        };
                        if target_delay != current_delay {
                            if let Err(err) = rollback_engine.set_input_delay_frames(target_delay) {
                                eprintln!("Netplay adaptive delay update failed: {err}");
                                *control_flow = ControlFlow::Exit;
                                return;
                            }
                            if let Some(stats) = netplay_stats.as_mut() {
                                stats.input_delay_frames = target_delay;
                                eprintln!(
                                    "[netplay] adaptive delay {} -> {} (rtt={:.1}ms jitter={:.1}ms)",
                                    current_delay,
                                    target_delay,
                                    stats.latest_rtt_ms_or_zero(),
                                    stats.jitter_ms
                                );
                            }
                        } else if let Some(stats) = netplay_stats.as_mut() {
                            stats.input_delay_frames = current_delay;
                        }

                        if netplay_hash_check_every > 0
                            && step.frame.is_multiple_of(netplay_hash_check_every)
                            && let Some(client) = netplay_client.as_ref()
                            && let Err(err) = client.send_hash(step.frame, step.state_hash)
                        {
                            eprintln!("Netplay send hash failed: {err}");
                            *control_flow = ControlFlow::Exit;
                            return;
                        }
                    }
                    Err(err) => {
                        eprintln!("Netplay rollback step failed: {err}");
                        *control_flow = ControlFlow::Exit;
                        return;
                    }
                }
            } else if let Err(err) = advance_core_for_host_frame(&mut core, step_mode) {
                eprintln!("CPU halted at PC ${:04X}: {err}", core.cpu_pc());
                *control_flow = ControlFlow::Exit;
                return;
            }

            let step_elapsed = step_start.elapsed();
            frame_index = frame_index.saturating_add(1);
            metrics.on_step(&core, step_elapsed, missed_deadline);
            if let Some(stats) = netplay_stats.as_ref() {
                metrics.on_netplay_stats(stats);
            }
            if trace_every_frames > 0 && frame_index.is_multiple_of(trace_every_frames) {
                let regs = core.cpu_snapshot();
                println!(
                    "frame={} ppu_frame={} pc=${:04X} a={:02X} x={:02X} y={:02X} ctrl1={:02X} ctrl2={:02X}",
                    frame_index,
                    core.ppu_frame_counter(),
                    regs.pc,
                    regs.a,
                    regs.x,
                    regs.y,
                    core.controller_bits(),
                    core.controller2_bits()
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
    let runtime_args = parse_runtime_args(&pass_through)?;

    let loaded_config_path = config_path.clone().or_else(|| {
        let default_path = PathBuf::from(DEFAULT_CONFIG_PATH);
        if default_path.exists() {
            Some(default_path)
        } else {
            None
        }
    });
    let config = NesConfig::load_or_default(config_path.as_deref())?;

    let rom_path = runtime_args
        .rom_path
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
    let netplay_enabled = runtime_args.netplay_enabled || config.netplay.enabled;
    let step_mode = if netplay_enabled {
        StepMode::Frame
    } else {
        match config.desktop.step_mode {
            StepModeConfig::Frame => StepMode::Frame,
            StepModeConfig::Cpu => StepMode::CpuBudget(cpu_steps_per_frame),
        }
    };

    let netplay = if netplay_enabled {
        let relay_addr = runtime_args
            .netplay_relay_addr
            .or_else(|| Some(config.netplay.relay_addr.clone()))
            .unwrap_or_default();
        let room = runtime_args
            .netplay_room
            .or_else(|| Some(config.netplay.room.clone()))
            .unwrap_or_default();
        let player = runtime_args.netplay_player.unwrap_or(config.netplay.player);
        let input_delay_frames = runtime_args
            .netplay_input_delay_frames
            .unwrap_or(config.netplay.input_delay_frames);
        let max_rollback_frames = runtime_args
            .netplay_max_rollback_frames
            .unwrap_or(config.netplay.max_rollback_frames);
        let hash_check_every_frames = runtime_args
            .netplay_hash_check_every_frames
            .unwrap_or(config.netplay.hash_check_every_frames);
        if room.trim().is_empty() {
            return Err("netplay room cannot be empty".to_owned());
        }
        Some(NetplayRuntimeConfig {
            relay_addr,
            room,
            player,
            input_delay_frames,
            max_rollback_frames,
            hash_check_every_frames,
        })
    } else {
        None
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
        mcp_enabled: runtime_args.mcp_enabled,
        mcp_bind_addr: runtime_args.mcp_bind_addr,
        netplay,
    })
}

fn parse_runtime_args(args: &[String]) -> Result<RuntimeArgs, String> {
    let mut parsed = RuntimeArgs {
        rom_path: None,
        mcp_enabled: false,
        mcp_bind_addr: DEFAULT_MCP_BIND_ADDR.to_owned(),
        netplay_enabled: false,
        netplay_relay_addr: None,
        netplay_room: None,
        netplay_player: None,
        netplay_input_delay_frames: None,
        netplay_max_rollback_frames: None,
        netplay_hash_check_every_frames: None,
    };
    let mut idx = 0_usize;
    while idx < args.len() {
        let arg = &args[idx];
        if arg == "--help" || arg == "-h" {
            return Err(format!(
                "Usage: nes-desktop [--config <path>] [--mcp-host] [--mcp-bind <addr>] [--netplay] [--netplay-relay <addr>] [--netplay-room <room>] [--netplay-player <1|2>] [--netplay-delay <frames>] [--netplay-max-rollback <frames>] [--netplay-hash-every <frames>] [rom_path]\nDefault config path: {DEFAULT_CONFIG_PATH}"
            ));
        }
        if arg == "--mcp-host" {
            parsed.mcp_enabled = true;
            idx += 1;
            continue;
        }
        if arg == "--netplay" {
            parsed.netplay_enabled = true;
            idx += 1;
            continue;
        }
        if arg == "--mcp-bind" {
            let Some(bind_addr) = args.get(idx + 1) else {
                return Err("missing value after --mcp-bind".to_owned());
            };
            parsed.mcp_bind_addr = bind_addr.clone();
            idx += 2;
            continue;
        }
        if arg == "--netplay-relay" {
            let Some(relay_addr) = args.get(idx + 1) else {
                return Err("missing value after --netplay-relay".to_owned());
            };
            parsed.netplay_relay_addr = Some(relay_addr.clone());
            idx += 2;
            continue;
        }
        if arg == "--netplay-room" {
            let Some(room) = args.get(idx + 1) else {
                return Err("missing value after --netplay-room".to_owned());
            };
            parsed.netplay_room = Some(room.clone());
            idx += 2;
            continue;
        }
        if arg == "--netplay-player" {
            let Some(player) = args.get(idx + 1) else {
                return Err("missing value after --netplay-player".to_owned());
            };
            parsed.netplay_player = Some(parse_u8_arg(player, "--netplay-player")?);
            idx += 2;
            continue;
        }
        if arg == "--netplay-delay" {
            let Some(delay) = args.get(idx + 1) else {
                return Err("missing value after --netplay-delay".to_owned());
            };
            parsed.netplay_input_delay_frames = Some(parse_u32_arg(delay, "--netplay-delay")?);
            idx += 2;
            continue;
        }
        if arg == "--netplay-max-rollback" {
            let Some(max_rollback) = args.get(idx + 1) else {
                return Err("missing value after --netplay-max-rollback".to_owned());
            };
            parsed.netplay_max_rollback_frames =
                Some(parse_u32_arg(max_rollback, "--netplay-max-rollback")?);
            idx += 2;
            continue;
        }
        if arg == "--netplay-hash-every" {
            let Some(hash_every) = args.get(idx + 1) else {
                return Err("missing value after --netplay-hash-every".to_owned());
            };
            parsed.netplay_hash_check_every_frames =
                Some(parse_u64_arg(hash_every, "--netplay-hash-every")?);
            idx += 2;
            continue;
        }
        if let Some(bind_addr) = arg.strip_prefix("--mcp-bind=") {
            if bind_addr.is_empty() {
                return Err("missing value after --mcp-bind=".to_owned());
            }
            parsed.mcp_bind_addr = bind_addr.to_owned();
            idx += 1;
            continue;
        }
        if let Some(relay_addr) = arg.strip_prefix("--netplay-relay=") {
            if relay_addr.is_empty() {
                return Err("missing value after --netplay-relay=".to_owned());
            }
            parsed.netplay_relay_addr = Some(relay_addr.to_owned());
            idx += 1;
            continue;
        }
        if let Some(room) = arg.strip_prefix("--netplay-room=") {
            if room.is_empty() {
                return Err("missing value after --netplay-room=".to_owned());
            }
            parsed.netplay_room = Some(room.to_owned());
            idx += 1;
            continue;
        }
        if let Some(player) = arg.strip_prefix("--netplay-player=") {
            if player.is_empty() {
                return Err("missing value after --netplay-player=".to_owned());
            }
            parsed.netplay_player = Some(parse_u8_arg(player, "--netplay-player")?);
            idx += 1;
            continue;
        }
        if let Some(delay) = arg.strip_prefix("--netplay-delay=") {
            if delay.is_empty() {
                return Err("missing value after --netplay-delay=".to_owned());
            }
            parsed.netplay_input_delay_frames = Some(parse_u32_arg(delay, "--netplay-delay")?);
            idx += 1;
            continue;
        }
        if let Some(max_rollback) = arg.strip_prefix("--netplay-max-rollback=") {
            if max_rollback.is_empty() {
                return Err("missing value after --netplay-max-rollback=".to_owned());
            }
            parsed.netplay_max_rollback_frames =
                Some(parse_u32_arg(max_rollback, "--netplay-max-rollback")?);
            idx += 1;
            continue;
        }
        if let Some(hash_every) = arg.strip_prefix("--netplay-hash-every=") {
            if hash_every.is_empty() {
                return Err("missing value after --netplay-hash-every=".to_owned());
            }
            parsed.netplay_hash_check_every_frames =
                Some(parse_u64_arg(hash_every, "--netplay-hash-every")?);
            idx += 1;
            continue;
        }
        if arg.starts_with("--") {
            return Err(format!(
                "unknown flag '{arg}'. Usage: nes-desktop [--config <path>] [--mcp-host] [--mcp-bind <addr>] [--netplay] [--netplay-relay <addr>] [--netplay-room <room>] [--netplay-player <1|2>] [--netplay-delay <frames>] [--netplay-max-rollback <frames>] [--netplay-hash-every <frames>] [rom_path]"
            ));
        }
        if parsed.rom_path.is_some() {
            return Err(
                "multiple ROM paths provided; expected at most one positional ROM path".to_owned(),
            );
        }
        parsed.rom_path = Some(arg.clone());
        idx += 1;
    }
    Ok(parsed)
}

fn parse_u8_arg(value: &str, flag: &str) -> Result<u8, String> {
    value
        .parse::<u8>()
        .map_err(|_| format!("{flag} must be an integer in [0, 255]"))
}

fn parse_u32_arg(value: &str, flag: &str) -> Result<u32, String> {
    value
        .parse::<u32>()
        .map_err(|_| format!("{flag} must be a non-negative integer"))
}

fn parse_u64_arg(value: &str, flag: &str) -> Result<u64, String> {
    value
        .parse::<u64>()
        .map_err(|_| format!("{flag} must be a non-negative integer"))
}

fn recommended_input_delay_frames(
    rtt_ms: Option<f64>,
    jitter_ms: f64,
    min_delay_frames: u32,
    max_delay_frames: u32,
    current_delay_frames: u32,
) -> u32 {
    if min_delay_frames >= max_delay_frames {
        return min_delay_frames;
    }
    let Some(rtt_ms) = rtt_ms else {
        return current_delay_frames;
    };

    let frame_time_ms = 1_000.0 / 60.0;
    let estimated_one_way_ms = (rtt_ms * 0.5) + (jitter_ms * 1.5);
    let raw_target = (estimated_one_way_ms / frame_time_ms).ceil() as u32 + 1;
    let target = raw_target.clamp(min_delay_frames, max_delay_frames);

    if target > current_delay_frames {
        target
    } else if target + 1 < current_delay_frames {
        current_delay_frames - 1
    } else {
        current_delay_frames
    }
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

fn connected_gamepad_ids(gilrs: &Gilrs) -> Vec<GamepadId> {
    gilrs
        .gamepads()
        .filter_map(|(id, gamepad)| gamepad.is_connected().then_some(id))
        .collect()
}

fn select_active_gamepad_ids(
    gilrs: &Gilrs,
    current: [Option<GamepadId>; 2],
) -> [Option<GamepadId>; 2] {
    let connected = connected_gamepad_ids(gilrs);
    let mut next = [None::<GamepadId>; 2];

    for player in 0..next.len() {
        if let Some(gamepad_id) = current[player]
            && connected.contains(&gamepad_id)
            && !next.contains(&Some(gamepad_id))
        {
            next[player] = Some(gamepad_id);
        }
    }

    for gamepad_id in connected {
        if next.iter().all(|slot| *slot != Some(gamepad_id))
            && let Some(slot) = next.iter_mut().find(|slot| slot.is_none())
        {
            *slot = Some(gamepad_id);
        }
    }

    next
}

fn sample_gamepad_bits(gilrs: &Gilrs, gamepad_id: GamepadId) -> u8 {
    let gamepad = gilrs.gamepad(gamepad_id);
    if !gamepad.is_connected() {
        return 0;
    }

    let mut bits = 0_u8;
    // Keep both common face layouts usable across Xbox/Switch-style controllers.
    if gamepad.is_pressed(GamepadButton::South) || gamepad.is_pressed(GamepadButton::East) {
        bits |= Button::A.bit_mask();
    }
    if gamepad.is_pressed(GamepadButton::West) || gamepad.is_pressed(GamepadButton::North) {
        bits |= Button::B.bit_mask();
    }
    if gamepad.is_pressed(GamepadButton::Select) {
        bits |= Button::Select.bit_mask();
    }
    if gamepad.is_pressed(GamepadButton::Start) {
        bits |= Button::Start.bit_mask();
    }

    let left_x = gamepad.value(GamepadAxis::LeftStickX);
    let left_y = gamepad.value(GamepadAxis::LeftStickY);
    if gamepad.is_pressed(GamepadButton::DPadUp) || left_y <= -GAMEPAD_AXIS_THRESHOLD {
        bits |= Button::Up.bit_mask();
    }
    if gamepad.is_pressed(GamepadButton::DPadDown) || left_y >= GAMEPAD_AXIS_THRESHOLD {
        bits |= Button::Down.bit_mask();
    }
    if gamepad.is_pressed(GamepadButton::DPadLeft) || left_x <= -GAMEPAD_AXIS_THRESHOLD {
        bits |= Button::Left.bit_mask();
    }
    if gamepad.is_pressed(GamepadButton::DPadRight) || left_x >= GAMEPAD_AXIS_THRESHOLD {
        bits |= Button::Right.bit_mask();
    }

    bits
}

fn controller_state_delta_for_player(previous: u8, current: u8, player2: bool) -> Vec<Command> {
    let mut commands = Vec::with_capacity(CONTROLLER_BUTTONS.len());
    for button in CONTROLLER_BUTTONS {
        let mask = button.bit_mask();
        match (previous & mask != 0, current & mask != 0) {
            (false, true) => {
                commands.push(if player2 {
                    Command::PressButton2(button)
                } else {
                    Command::PressButton(button)
                });
            }
            (true, false) => {
                commands.push(if player2 {
                    Command::ReleaseButton2(button)
                } else {
                    Command::ReleaseButton(button)
                });
            }
            _ => {}
        }
    }
    commands
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
    use super::{
        DEFAULT_MCP_BIND_ADDR, RuntimeArgs, controller_state_delta_for_player, parse_runtime_args,
        recommended_input_delay_frames,
    };
    use nes_core::{Button, Command};

    #[test]
    fn parse_runtime_args_accepts_mcp_host_and_bind_flags() {
        let args = vec![
            "--mcp-host".to_owned(),
            "--mcp-bind".to_owned(),
            "127.0.0.1:7777".to_owned(),
            "game.nes".to_owned(),
        ];
        let parsed = parse_runtime_args(&args).expect("parse args");
        assert_eq!(parsed.rom_path.as_deref(), Some("game.nes"));
        assert!(parsed.mcp_enabled);
        assert_eq!(parsed.mcp_bind_addr, "127.0.0.1:7777");
    }

    #[test]
    fn parse_runtime_args_accepts_equals_bind_form() {
        let args = vec!["--mcp-bind=127.0.0.1:7000".to_owned()];
        let parsed = parse_runtime_args(&args).expect("parse args");
        assert!(parsed.rom_path.is_none());
        assert!(!parsed.mcp_enabled);
        assert_eq!(parsed.mcp_bind_addr, "127.0.0.1:7000");
    }

    #[test]
    fn parse_runtime_args_defaults_bind_when_flag_absent() {
        let args = vec!["game.nes".to_owned()];
        let parsed = parse_runtime_args(&args).expect("parse args");
        assert_eq!(parsed.rom_path.as_deref(), Some("game.nes"));
        assert!(!parsed.mcp_enabled);
        assert_eq!(parsed.mcp_bind_addr, DEFAULT_MCP_BIND_ADDR);
    }

    #[test]
    fn parse_runtime_args_rejects_unknown_flags() {
        let args = vec!["--bogus".to_owned()];
        let err = parse_runtime_args(&args).expect_err("unknown flag should fail");
        assert!(err.contains("unknown flag"));
    }

    #[test]
    fn parse_runtime_args_accepts_netplay_flags() {
        let args = vec![
            "--netplay".to_owned(),
            "--netplay-relay=relay.example:4545".to_owned(),
            "--netplay-room".to_owned(),
            "river-city".to_owned(),
            "--netplay-player".to_owned(),
            "2".to_owned(),
            "--netplay-delay=3".to_owned(),
            "--netplay-max-rollback".to_owned(),
            "360".to_owned(),
            "--netplay-hash-every".to_owned(),
            "90".to_owned(),
        ];
        let parsed = parse_runtime_args(&args).expect("parse args");
        let expected = RuntimeArgs {
            rom_path: None,
            mcp_enabled: false,
            mcp_bind_addr: DEFAULT_MCP_BIND_ADDR.to_owned(),
            netplay_enabled: true,
            netplay_relay_addr: Some("relay.example:4545".to_owned()),
            netplay_room: Some("river-city".to_owned()),
            netplay_player: Some(2),
            netplay_input_delay_frames: Some(3),
            netplay_max_rollback_frames: Some(360),
            netplay_hash_check_every_frames: Some(90),
        };
        assert_eq!(parsed, expected);
    }

    #[test]
    fn adaptive_delay_reacts_to_rtt_and_jitter() {
        let increased = recommended_input_delay_frames(Some(96.0), 12.0, 1, 12, 2);
        assert!(
            increased >= 4,
            "expected higher delay for higher RTT+jitter"
        );

        let unchanged = recommended_input_delay_frames(Some(96.0), 12.0, 1, 12, increased);
        assert!(
            unchanged == increased || unchanged + 1 == increased,
            "hysteresis should avoid abrupt downshifts"
        );
    }

    #[test]
    fn adaptive_delay_uses_current_when_no_rtt_sample() {
        let delay = recommended_input_delay_frames(None, 0.0, 1, 12, 3);
        assert_eq!(delay, 3);
    }

    #[test]
    fn controller_state_delta_emits_press_and_release() {
        let press = controller_state_delta_for_player(
            0,
            Button::A.bit_mask() | Button::Right.bit_mask(),
            false,
        );
        assert_eq!(
            press,
            vec![
                Command::PressButton(Button::A),
                Command::PressButton(Button::Right)
            ]
        );

        let release = controller_state_delta_for_player(
            Button::A.bit_mask() | Button::B.bit_mask(),
            Button::B.bit_mask(),
            false,
        );
        assert_eq!(release, vec![Command::ReleaseButton(Button::A)]);
    }

    #[test]
    fn controller_state_delta_for_player2_uses_player2_commands() {
        let press = controller_state_delta_for_player(0, Button::A.bit_mask(), true);
        assert_eq!(press, vec![Command::PressButton2(Button::A)]);

        let release = controller_state_delta_for_player(Button::Start.bit_mask(), 0, true);
        assert_eq!(release, vec![Command::ReleaseButton2(Button::Start)]);
    }
}
