use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::path::PathBuf;
use std::time::{Duration, Instant};

#[cfg(feature = "nova")]
mod auto_player;
#[cfg(feature = "mcp-host")]
mod mcp_host;
mod netplay;

use comfy_table::{Cell, Color as TableColor, Table};
use crossterm::style::{Color, Stylize};
use gilrs::{Axis as GamepadAxis, Button as GamepadButton, GamepadId, Gilrs};
use nes_config::{
    DEFAULT_CONFIG_PATH, NesConfig, StepModeConfig, normalize_nonzero_u32, normalize_nonzero_u64,
    parse_config_path_arg,
};
use nes_core::{
    AUDIO_SAMPLE_RATE, Button, Command, FRAME_HEIGHT, FRAME_RGBA_BYTES, FRAME_WIDTH, NesCore,
};
use nes_desktop::app::{map_key_event_to_button_bit, map_key_event_to_command};
use nes_desktop::manual_state::{load_state_file, quicksave_path_for_rom, save_state_file};
use nes_desktop::rta::{
    CalibrationRecorder, DEFAULT_RTA_PROFILES_DIR, DEFAULT_RTA_RUNS_DIR, ForbiddenAction,
    ProfileStatus, RtaEvent, RtaManager, RtaProfile, compute_rom_hash, load_profiles,
    select_profile,
};
use nes_netplay::{HashComparison, RollbackConfig, RollbackEngine, ServerMessage};
use nes_rewind::worker::{TimeMachine, TimeMachineConfig};
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
    rta: Option<RtaRuntimeConfig>,
    #[cfg(feature = "nova")]
    auto_player_enabled: bool,
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
    rta_enabled: bool,
    rta_profile_id: Option<String>,
    rta_profiles_dir: Option<String>,
    rta_runs_dir: Option<String>,
    rta_calibrate: bool,
    #[cfg(feature = "nova")]
    auto_player_enabled: bool,
}

#[derive(Debug, Clone)]
struct RtaRuntimeConfig {
    profile_id_override: Option<String>,
    profiles_dir: PathBuf,
    runs_dir: PathBuf,
    calibrate: bool,
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

trait AudioSinkControl {
    fn queue_len(&self) -> usize;
    fn append_i16(&self, samples: Vec<i16>);
    fn clear(&self);
    fn stop(&self);
}

struct RodioSinkAdapter {
    inner: Sink,
}

impl AudioSinkControl for RodioSinkAdapter {
    fn queue_len(&self) -> usize {
        self.inner.len()
    }

    fn append_i16(&self, samples: Vec<i16>) {
        self.inner.append(SamplesBuffer::new(
            AUDIO_CHANNELS,
            AUDIO_SAMPLE_RATE,
            samples,
        ));
    }

    fn clear(&self) {
        self.inner.clear();
    }

    fn stop(&self) {
        self.inner.stop();
    }
}

struct AudioOutput {
    sink: Box<dyn AudioSinkControl>,
    _stream: Option<OutputStream>,
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

#[derive(Debug, Clone, Copy, PartialEq)]
struct MetricsSnapshot {
    wall_fps: f64,
    emu_fps: f64,
    avg_step_ms: f64,
    avg_render_ms: f64,
}

fn compute_metrics_snapshot(
    report_frames: u64,
    elapsed_secs: f64,
    report_start_ppu_frame: u64,
    ppu_now: u64,
    step_work: Duration,
    render_work: Duration,
) -> Option<MetricsSnapshot> {
    if report_frames == 0 || elapsed_secs <= f64::EPSILON {
        return None;
    }
    let wall_fps = report_frames as f64 / elapsed_secs;
    let emu_fps = ppu_now.saturating_sub(report_start_ppu_frame) as f64 / elapsed_secs;
    let avg_step_ms = step_work.as_secs_f64() * 1_000.0 / report_frames as f64;
    let avg_render_ms = render_work.as_secs_f64() * 1_000.0 / report_frames as f64;
    Some(MetricsSnapshot {
        wall_fps,
        emu_fps,
        avg_step_ms,
        avg_render_ms,
    })
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
        let ppu_now = core.ppu_frame_counter();
        let Some(snapshot) = compute_metrics_snapshot(
            self.report_frames,
            elapsed,
            self.report_start_ppu_frame,
            ppu_now,
            self.step_work,
            self.render_work,
        ) else {
            return;
        };

        println!(
            "[metrics] wall_fps={:.1} emu_fps={:.1} avg_step_ms={:.2} avg_render_ms={:.2} late_frames={} pc_stall_frames={} unchanged_frames={} audio_peak_q={} audio_drop_chunks={} net_rtt_ms={:.1} net_jitter_ms={:.1} net_rollbacks={} net_max_rb={} net_desyncs={} net_delay_frames={}",
            snapshot.wall_fps,
            snapshot.emu_fps,
            snapshot.avg_step_ms,
            snapshot.avg_render_ms,
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
    fn new_with_sink(sink: Box<dyn AudioSinkControl>, stream: Option<OutputStream>) -> Self {
        Self {
            sink,
            _stream: stream,
        }
    }

    fn try_new() -> Result<Self, String> {
        let (stream, handle) = OutputStream::try_default()
            .map_err(|err| format!("Audio output init failed: {err}"))?;
        let sink =
            Sink::try_new(&handle).map_err(|err| format!("Audio sink init failed: {err}"))?;
        let sink_adapter = RodioSinkAdapter { inner: sink };
        Ok(Self::new_with_sink(Box::new(sink_adapter), Some(stream)))
    }

    fn queue_samples(&self, samples: Vec<i16>) -> bool {
        if self.sink.queue_len() >= MAX_AUDIO_QUEUE_CHUNKS {
            return false;
        }
        self.sink.append_i16(samples);
        true
    }

    fn queue_len(&self) -> usize {
        self.sink.queue_len()
    }

    fn clear(&self) {
        self.sink.clear();
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum KeyboardDecision {
    Exit,
    ManualSaveState,
    ManualLoadState,
    SetRewindHeld(bool),
    RtaManualSplit,
    RtaFinish,
    UpdateKeyboardBits { mask: u8, pressed: bool },
    ExecuteCore(Command),
    Noop,
}

#[derive(Debug, Clone, Copy)]
enum FrameDecision {
    WaitUntil(Instant),
    Step {
        missed_deadline: bool,
        next_deadline: Instant,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WindowEventDecision {
    CloseRequested,
    KeyboardInput {
        key: Option<VirtualKeyCode>,
        pressed: bool,
    },
    Resized {
        width: u32,
        height: u32,
    },
    ScaleFactorChanged {
        width: u32,
        height: u32,
    },
    Ignore,
}

fn classify_window_event(event: &WindowEvent<'_>) -> WindowEventDecision {
    match event {
        WindowEvent::CloseRequested => WindowEventDecision::CloseRequested,
        WindowEvent::KeyboardInput { input, .. } => WindowEventDecision::KeyboardInput {
            key: input.virtual_keycode,
            pressed: element_state_pressed(input.state),
        },
        WindowEvent::Resized(size) => WindowEventDecision::Resized {
            width: size.width,
            height: size.height,
        },
        WindowEvent::ScaleFactorChanged { new_inner_size, .. } => {
            WindowEventDecision::ScaleFactorChanged {
                width: new_inner_size.width,
                height: new_inner_size.height,
            }
        }
        _ => WindowEventDecision::Ignore,
    }
}

struct KeyboardContext {
    rollback_enabled: bool,
    rta_enabled: bool,
    rta_calibrate: bool,
}

fn classify_keyboard_input(
    key: VirtualKeyCode,
    pressed: bool,
    ctx: KeyboardContext,
) -> KeyboardDecision {
    if key == VirtualKeyCode::Escape && pressed {
        return KeyboardDecision::Exit;
    }
    if pressed && key == VirtualKeyCode::F5 {
        return KeyboardDecision::ManualSaveState;
    }
    if pressed && key == VirtualKeyCode::F8 {
        return KeyboardDecision::ManualLoadState;
    }
    if key == VirtualKeyCode::R {
        return KeyboardDecision::SetRewindHeld(pressed);
    }
    if ctx.rta_enabled && pressed && key == VirtualKeyCode::F9 {
        return KeyboardDecision::RtaManualSplit;
    }
    if ctx.rta_enabled && ctx.rta_calibrate && pressed && key == VirtualKeyCode::F10 {
        return KeyboardDecision::RtaFinish;
    }

    let Some(key_code) = map_virtual_keycode(key) else {
        return KeyboardDecision::Noop;
    };

    if ctx.rollback_enabled {
        if let Some(mask) = map_key_event_to_button_bit(key_code) {
            KeyboardDecision::UpdateKeyboardBits { mask, pressed }
        } else {
            KeyboardDecision::Noop
        }
    } else if let Some(mapped) = map_key_event_to_command(key_code, pressed) {
        KeyboardDecision::ExecuteCore(mapped.core)
    } else {
        KeyboardDecision::Noop
    }
}

fn command_marks_rta_invalidation(command: Command) -> Option<ForbiddenAction> {
    match command {
        Command::StepCpu | Command::StepScanline | Command::StepFrame => {
            Some(ForbiddenAction::FrameStep)
        }
        _ => None,
    }
}

fn evaluate_frame_deadline(now: Instant, next_frame_deadline: Instant) -> FrameDecision {
    if now < next_frame_deadline {
        FrameDecision::WaitUntil(next_frame_deadline)
    } else {
        FrameDecision::Step {
            missed_deadline: now > next_frame_deadline,
            next_deadline: now + TARGET_FRAME_TIME,
        }
    }
}

fn scaled_window_dimensions(window_scale: u32) -> (f64, f64) {
    (
        f64::from(FRAME_WIDTH as u32 * window_scale),
        f64::from(FRAME_HEIGHT as u32 * window_scale),
    )
}

fn gamepad_assignments_changed(
    next: [Option<GamepadId>; 2],
    current: [Option<GamepadId>; 2],
) -> bool {
    next != current
}

fn gamepad_slot_changed(
    next: [Option<GamepadId>; 2],
    current: [Option<GamepadId>; 2],
    player: usize,
) -> bool {
    next[player] != current[player]
}

fn element_state_pressed(state: ElementState) -> bool {
    state == ElementState::Pressed
}

fn should_resume_after_rewind_hold(held: bool) -> bool {
    !held
}

fn release_all_buttons(core: &mut NesCore) {
    for &button in &CONTROLLER_BUTTONS {
        let _ = core.execute(Command::ReleaseButton(button));
        let _ = core.execute(Command::ReleaseButton2(button));
    }
}

fn track_keyboard_bits_for_key(key: VirtualKeyCode, pressed: bool, keyboard_bits: &mut u8) {
    if let Some(key_code) = map_virtual_keycode(key)
        && let Some(mask) = map_key_event_to_button_bit(key_code)
    {
        *keyboard_bits = update_button_bits(*keyboard_bits, mask, pressed);
    }
}

fn resync_restored_inputs(
    core: &mut NesCore,
    keyboard_bits: u8,
    gamepad_bits: &mut [u8; 2],
) -> Result<(), String> {
    release_all_buttons(core);
    *gamepad_bits = [0; 2];
    apply_gamepad_delta_commands(core, 0, keyboard_bits, false)
}

fn is_player_two_slot(player_index: usize) -> bool {
    player_index == 1
}

fn merge_local_input_bits(keyboard_bits: u8, local_gamepad_bits: u8) -> u8 {
    keyboard_bits | local_gamepad_bits
}

fn netplay_feature_enabled(runtime_flag: bool, config_flag: bool) -> bool {
    runtime_flag || config_flag
}

fn should_log_rollback(distance: u64) -> bool {
    distance > 0
}

fn should_update_input_delay(target_delay: u32, current_delay: u32) -> bool {
    target_delay != current_delay
}

fn should_trace_frame(trace_every_frames: u64, frame_index: u64) -> bool {
    trace_every_frames != 0 && frame_index != 0 && frame_index.is_multiple_of(trace_every_frames)
}

fn audio_queue_dropped(queued: bool) -> bool {
    !queued
}

fn should_capture_frame(every_n_frames: u64, frame_index: u64) -> bool {
    every_n_frames != 0 && frame_index.is_multiple_of(every_n_frames)
}

fn compute_local_netplay_bits(gamepad_bits: [u8; 2], local_player: u8) -> u8 {
    let local_slot = usize::from(local_player.saturating_sub(1));
    gamepad_bits.get(local_slot).copied().unwrap_or_else(|| {
        gamepad_bits
            .iter()
            .copied()
            .find(|bits| *bits != 0)
            .unwrap_or(0)
    })
}

fn should_send_netplay_hash(hash_check_every: u64, frame: u64) -> bool {
    hash_check_every != 0 && frame != 0 && frame.is_multiple_of(hash_check_every)
}

fn schedule_netplay_ping(
    now: Instant,
    next_ping_at: &mut Instant,
    ping_nonce: &mut u64,
    pending_pings: &mut BTreeMap<u64, Instant>,
    ping_interval: Duration,
    max_pending: usize,
) -> Option<u64> {
    if now < *next_ping_at {
        return None;
    }

    let nonce = *ping_nonce;
    *ping_nonce = ping_nonce.wrapping_add(1);
    pending_pings.insert(nonce, now);
    while pending_pings.len() > max_pending {
        if let Some(oldest_nonce) = pending_pings.keys().next().copied() {
            pending_pings.remove(&oldest_nonce);
        }
    }
    *next_ping_at = now + ping_interval;
    Some(nonce)
}

fn update_button_bits(current: u8, mask: u8, pressed: bool) -> u8 {
    if pressed {
        current | mask
    } else {
        current & !mask
    }
}

fn apply_gamepad_delta_commands(
    core: &mut NesCore,
    previous_bits: u8,
    next_bits: u8,
    player2: bool,
) -> Result<(), String> {
    for command in controller_state_delta_for_player(previous_bits, next_bits, player2) {
        core.execute(command)
            .map_err(|err| format!("Gamepad command failed: {err}"))?;
    }
    Ok(())
}

fn handle_netplay_server_message(
    message: ServerMessage,
    rollback_engine: &mut RollbackEngine,
    netplay_local_player: u8,
    netplay_stats: &mut Option<NetplayRuntimeStats>,
    netplay_pending_pings: &mut BTreeMap<u64, Instant>,
) -> Result<(), String> {
    match message {
        ServerMessage::PeerInput {
            player,
            frame,
            bits,
        } => {
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
                            frame, state_hash
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
            return Err(format!("[netplay] relay error: {message}"));
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
    Ok(())
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
    let rom_bytes = fs::read(&rom_path).map_err(|err| format_rom_read_error(&rom_path, &err))?;

    let mut core = NesCore::new();
    let info = core
        .load_ines_rom(&rom_bytes)
        .map_err(|err| format!("Failed to load ROM: {err}"))?;
    let step_mode = runtime.step_mode;
    let rom_hash = compute_rom_hash(&rom_bytes);
    let manual_state_path =
        quicksave_path_for_rom(std::path::Path::new(&runtime.rom_path), &rom_hash);
    let mut rta_manager = if let Some(rta_config) = runtime.rta.as_ref() {
        let profiles = load_profiles(&rta_config.profiles_dir)?;
        let profile = if rta_config.calibrate {
            match select_profile(
                &profiles,
                &rom_hash,
                rta_config.profile_id_override.as_deref(),
                true,
            ) {
                Ok(selection) => selection.selected.profile,
                Err(err) => {
                    if let Some(profile_id) = rta_config.profile_id_override.as_ref() {
                        eprintln!(
                            "[rta] calibration creating profile template '{}' ({err})",
                            profile_id
                        );
                        RtaProfile {
                            id: profile_id.clone(),
                            rom_hashes: vec![rom_hash.clone()],
                            status: ProfileStatus::Published,
                            ..RtaProfile::default()
                        }
                    } else {
                        return Err(format!(
                            "RTA calibration requires --rta-profile <id> when no existing profile matches ROM hash {rom_hash}: {err}"
                        ));
                    }
                }
            }
        } else {
            select_profile(
                &profiles,
                &rom_hash,
                rta_config.profile_id_override.as_deref(),
                false,
            )
            .map_err(|err| {
                format!(
                    "Failed to enter RTA mode for ROM hash {rom_hash}: {err}. Provide --rta-profile <id> to override."
                )
            })?
            .selected
            .profile
        };
        let calibration = if rta_config.calibrate {
            Some(CalibrationRecorder::new(profile.id.clone()))
        } else {
            None
        };
        Some(RtaManager::new(
            profile,
            rom_hash.clone(),
            rta_config.runs_dir.clone(),
            calibration,
        ))
    } else {
        None
    };

    let mut table = Table::new();
    table.set_header(vec![
        Cell::new("Setting").fg(TableColor::Cyan),
        Cell::new("Value").fg(TableColor::White),
    ]);

    table.add_row(vec![
        Cell::new("ROM Path"),
        Cell::new(&rom_path).fg(TableColor::Green),
    ]);
    table.add_row(vec![
        Cell::new("ROM Info"),
        Cell::new(format!(
            "Mapper {}, PRG {} bytes, reset vector ${:04X}",
            info.mapper_id, info.prg_rom_bytes, info.reset_pc
        )),
    ]);
    if let Some(config_path) = runtime.loaded_config_path.as_ref() {
        table.add_row(vec![
            Cell::new("Config"),
            Cell::new(config_path.display().to_string()),
        ]);
    }
    table.add_row(vec![
        Cell::new("Controls"),
        Cell::new(
            "keyboard Z=A, X=B, Enter=Start, RightShift=Select, Arrows=D-pad, R=Rewind, F5=Save, F8=Load, Esc=Quit",
        ),
    ]);
    table.add_row(vec![
        Cell::new("Gamepad"),
        Cell::new("face buttons=A/B, Start/Select, D-pad or left stick"),
    ]);
    match step_mode {
        StepMode::Frame => {
            table.add_row(vec![Cell::new("Step Mode"), Cell::new("frame")]);
        }
        StepMode::CpuBudget(steps) => {
            table.add_row(vec![
                Cell::new("Step Mode"),
                Cell::new(format!("cpu ({steps} instructions/frame)")),
            ]);
        }
    }
    if let Some(netplay) = runtime.netplay.as_ref() {
        table.add_row(vec![
            Cell::new("Netplay"),
            Cell::new(format!(
                "relay={} room='{}' player={} delay={} rollback={} hash_every={}",
                netplay.relay_addr,
                netplay.room,
                netplay.player,
                netplay.input_delay_frames,
                netplay.max_rollback_frames,
                netplay.hash_check_every_frames
            )),
        ]);
    }
    if let Some(rta) = rta_manager.as_ref() {
        table.add_row(vec![
            Cell::new("RTA"),
            Cell::new(format!(
                "enabled profile='{}' calibrate={}",
                rta.profile_id(),
                rta.is_calibrating()
            )),
        ]);
    }
    #[cfg(feature = "nova")]
    {
        if runtime.auto_player_enabled {
            table.add_row(vec![
                Cell::new("Nova"),
                Cell::new("Auto Player Chaos Fuzzing Enabled"),
            ]);
        }
    }

    println!("{}", "nes-desktop".with(Color::Cyan).bold());
    println!("{table}\n");
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
    let (window_width, window_height) = scaled_window_dimensions(runtime.window_scale);
    let window = WindowBuilder::new()
        .with_title("nes-desktop")
        .with_inner_size(LogicalSize::new(window_width, window_height))
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
        let connected = connected_gamepad_ids(
            gilrs_state
                .gamepads()
                .map(|(id, gamepad)| (id, gamepad.is_connected())),
        );
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

    let mut time_machine = TimeMachine::new(TimeMachineConfig::default());
    let mut rewind_held = false;

    #[cfg(feature = "nova")]
    let mut auto_player = if runtime.auto_player_enabled {
        Some(crate::auto_player::AutoPlayer::new())
    } else {
        None
    };

    event_loop.run(move |event, _, control_flow| match event {
        Event::WindowEvent { event, .. } => match classify_window_event(&event) {
            WindowEventDecision::CloseRequested => {
                if let Some(rta) = rta_manager.as_mut() {
                    if rta.is_calibrating() && rta.is_active() {
                        let _ = rta.force_finish(frame_index, Instant::now());
                    }
                    let _ = rta.write_artifacts_if_finished();
                    if let Some(rta_config) = runtime.rta.as_ref() {
                        let _ = rta.write_calibration_draft(&rta_config.profiles_dir);
                    }
                }
                *control_flow = ControlFlow::Exit;
            }
            WindowEventDecision::KeyboardInput { key, pressed } => {
                let Some(key) = key else {
                    return;
                };
                track_keyboard_bits_for_key(key, pressed, &mut keyboard_bits);
                match classify_keyboard_input(
                    key,
                    pressed,
                    KeyboardContext {
                        rollback_enabled: rollback.is_some(),
                        rta_enabled: rta_manager.is_some(),
                        rta_calibrate: rta_manager.as_ref().is_some_and(|manager| manager.is_calibrating()),
                    },
                ) {
                    KeyboardDecision::Exit => {
                        if let Some(rta) = rta_manager.as_mut() {
                            if rta.is_calibrating() && rta.is_active() {
                                let _ = rta.force_finish(frame_index, Instant::now());
                            }
                            let _ = rta.write_artifacts_if_finished();
                            if let Some(rta_config) = runtime.rta.as_ref() {
                                let _ = rta.write_calibration_draft(&rta_config.profiles_dir);
                            }
                        }
                        *control_flow = ControlFlow::Exit;
                    }
                    KeyboardDecision::ManualSaveState => {
                        if rollback.is_some() {
                            eprintln!(
                                "[state] manual save-state is unavailable while netplay/rollback is active"
                            );
                            return;
                        }
                        if let Some(rta) = rta_manager.as_mut() {
                            let _ = rta.mark_forbidden_action(
                                ForbiddenAction::SaveLoad,
                                frame_index,
                                Instant::now(),
                            );
                        }
                        let snapshot = core.save_state();
                        match save_state_file(&manual_state_path, &rom_hash, &snapshot) {
                            Ok(()) => {
                                eprintln!("[state] saved {}", manual_state_path.display());
                            }
                            Err(err) => {
                                eprintln!("[state] save failed: {err}");
                            }
                        }
                    }
                    KeyboardDecision::ManualLoadState => {
                        if rollback.is_some() {
                            eprintln!(
                                "[state] manual load-state is unavailable while netplay/rollback is active"
                            );
                            return;
                        }
                        if let Some(rta) = rta_manager.as_mut() {
                            let _ = rta.mark_forbidden_action(
                                ForbiddenAction::SaveLoad,
                                frame_index,
                                Instant::now(),
                            );
                        }
                        match load_state_file(&manual_state_path, &rom_hash) {
                            Ok(snapshot) => {
                                core.load_state(&snapshot);
                                if let Err(err) =
                                    resync_restored_inputs(&mut core, keyboard_bits, &mut gamepad_bits)
                                {
                                    eprintln!("Input resync failed: {err}");
                                    *control_flow = ControlFlow::Exit;
                                    return;
                                }
                                if let Some(output) = audio_output.as_ref() {
                                    output.clear();
                                }
                                rewind_held = false;
                                time_machine = TimeMachine::new(TimeMachineConfig::default());
                                time_machine.record_frame(&core);
                                metrics = PerfMetrics::new(
                                    runtime.metrics_enabled,
                                    runtime.metrics_every_frames,
                                    core.ppu_frame_counter(),
                                );
                                eprintln!("[state] loaded {}", manual_state_path.display());
                            }
                            Err(err) => {
                                eprintln!("[state] load failed: {err}");
                            }
                        }
                    }
                    KeyboardDecision::SetRewindHeld(held) => {
                        // R: hold to rewind, release to resume.
                        rewind_held = held;
                        if held
                            && let Some(rta) = rta_manager.as_mut()
                        {
                            let _ = rta.mark_forbidden_action(
                                ForbiddenAction::Rewind,
                                frame_index,
                                Instant::now(),
                            );
                        }
                        if should_resume_after_rewind_hold(held) {
                            time_machine.resume();
                            // The restored snapshot's controller bits may reflect buttons
                            // held at that historical frame. Release both pads so the
                            // core's latch matches the host's live input state going forward.
                            if let Err(err) =
                                resync_restored_inputs(&mut core, keyboard_bits, &mut gamepad_bits)
                            {
                                eprintln!("Input resync failed: {err}");
                                *control_flow = ControlFlow::Exit;
                            }
                        }
                    }
                    KeyboardDecision::RtaManualSplit => {
                        if let Some(rta) = rta_manager.as_mut() {
                            let _ = rta.manual_split(frame_index, Instant::now());
                        }
                    }
                    KeyboardDecision::RtaFinish => {
                        if let Some(rta) = rta_manager.as_mut() {
                            let _ = rta.force_finish(frame_index, Instant::now());
                            let _ = rta.write_artifacts_if_finished();
                            if let Some(rta_config) = runtime.rta.as_ref() {
                                let _ = rta.write_calibration_draft(&rta_config.profiles_dir);
                            }
                        }
                    }
                    KeyboardDecision::UpdateKeyboardBits { mask, pressed } => {
                        keyboard_bits = update_button_bits(keyboard_bits, mask, pressed);
                    }
                    KeyboardDecision::ExecuteCore(command) => {
                        if let Some(action) = command_marks_rta_invalidation(command)
                            && let Some(rta) = rta_manager.as_mut()
                        {
                            let _ = rta.mark_forbidden_action(action, frame_index, Instant::now());
                        }
                        if let Err(err) = core.execute(command) {
                            eprintln!("Input command failed: {err}");
                            *control_flow = ControlFlow::Exit;
                        }
                    }
                    KeyboardDecision::Noop => {}
                }
            }
            WindowEventDecision::Resized { width, height } => {
                if let Err(err) = pixels.resize_surface(width, height) {
                    eprintln!("Surface resize failed: {err}");
                    *control_flow = ControlFlow::Exit;
                }
            }
            WindowEventDecision::ScaleFactorChanged { width, height } => {
                if let Err(err) = pixels.resize_surface(width, height) {
                    eprintln!("Scale-factor resize failed: {err}");
                    *control_flow = ControlFlow::Exit;
                }
            }
            WindowEventDecision::Ignore => {}
        },
        Event::MainEventsCleared => {
            #[cfg(feature = "mcp-host")]
            if let Some(host) = mcp_host.as_ref() {
                host.drain(&mut core);
            }

            if let Some(gilrs_state) = gilrs.as_mut() {
                while gilrs_state.next_event().is_some() {}
                let connected = connected_gamepad_ids(
                    gilrs_state
                        .gamepads()
                        .map(|(id, gamepad)| (id, gamepad.is_connected())),
                );
                let next_active = select_active_gamepad_ids(&connected, active_gamepads);
                if gamepad_assignments_changed(next_active, active_gamepads) {
                    for player in 0..active_gamepads.len() {
                        if gamepad_slot_changed(next_active, active_gamepads, player) {
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
                        .map(|gamepad_id| {
                            let gamepad = gilrs_state.gamepad(gamepad_id);
                            gamepad_snapshot_to_bits(GamepadSnapshot {
                                connected: gamepad.is_connected(),
                                south_pressed: gamepad.is_pressed(GamepadButton::South),
                                east_pressed: gamepad.is_pressed(GamepadButton::East),
                                west_pressed: gamepad.is_pressed(GamepadButton::West),
                                north_pressed: gamepad.is_pressed(GamepadButton::North),
                                select_pressed: gamepad.is_pressed(GamepadButton::Select),
                                start_pressed: gamepad.is_pressed(GamepadButton::Start),
                                dpad_up_pressed: gamepad.is_pressed(GamepadButton::DPadUp),
                                dpad_down_pressed: gamepad.is_pressed(GamepadButton::DPadDown),
                                dpad_left_pressed: gamepad.is_pressed(GamepadButton::DPadLeft),
                                dpad_right_pressed: gamepad.is_pressed(GamepadButton::DPadRight),
                                left_x: gamepad.value(GamepadAxis::LeftStickX),
                                left_y: gamepad.value(GamepadAxis::LeftStickY),
                            })
                        })
                        .unwrap_or_default();
                    if rollback.is_none()
                        && let Err(err) = apply_gamepad_delta_commands(
                            &mut core,
                            gamepad_bits[player],
                            next_gamepad_bits,
                            is_player_two_slot(player),
                        )
                    {
                        eprintln!("{err}");
                        *control_flow = ControlFlow::Exit;
                        return;
                    }
                    gamepad_bits[player] = next_gamepad_bits;
                }
            }

            let now = Instant::now();
            let missed_deadline = match evaluate_frame_deadline(now, next_frame_deadline) {
                FrameDecision::WaitUntil(deadline) => {
                    *control_flow = ControlFlow::WaitUntil(deadline);
                    return;
                }
                FrameDecision::Step {
                    missed_deadline,
                    next_deadline,
                } => {
                    next_frame_deadline = next_deadline;
                    missed_deadline
                }
            };
            let step_start = Instant::now();

            #[cfg(feature = "nova")]
            if let Some(player) = auto_player.as_mut() {
                player.step(&mut core);
            }

            if let Some(rollback_engine) = rollback.as_mut() {
                let local_gamepad_bits =
                    compute_local_netplay_bits(gamepad_bits, netplay_local_player);
                let scheduled = rollback_engine
                    .schedule_local_input(merge_local_input_bits(keyboard_bits, local_gamepad_bits));
                if let Some(client) = netplay_client.as_ref()
                    && let Err(err) = client.send_input(scheduled.frame, scheduled.bits)
                {
                    eprintln!("Netplay send input failed: {err}");
                    *control_flow = ControlFlow::Exit;
                    return;
                }
                if let Some(client) = netplay_client.as_ref() {
                    if let Some(nonce) = schedule_netplay_ping(
                        now,
                        &mut netplay_next_ping_at,
                        &mut netplay_ping_nonce,
                        &mut netplay_pending_pings,
                        NETPLAY_PING_INTERVAL,
                        128,
                    ) && let Err(err) = client.send_ping(nonce)
                    {
                        eprintln!("Netplay send ping failed: {err}");
                        *control_flow = ControlFlow::Exit;
                        return;
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
                        if let Err(err) = handle_netplay_server_message(
                            message,
                            rollback_engine,
                            netplay_local_player,
                            &mut netplay_stats,
                            &mut netplay_pending_pings,
                        ) {
                            eprintln!("{err}");
                            *control_flow = ControlFlow::Exit;
                            return;
                        }
                    }
                }

                match rollback_engine.advance_frame(&mut core) {
                    Ok(step) => {
                        if should_log_rollback(step.rollback_distance) {
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
                        if should_update_input_delay(target_delay, current_delay) {
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

                        if should_send_netplay_hash(netplay_hash_check_every, step.frame)
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
            } else if rewind_held {
                time_machine.rewind_step(&mut core);
            } else if let Err(err) = advance_core_for_host_frame(&mut core, step_mode) {
                eprintln!("CPU halted at PC ${:04X}: {err}", core.cpu_pc());
                *control_flow = ControlFlow::Exit;
                return;
            } else {
                time_machine.record_frame(&core);
            }

            let step_elapsed = step_start.elapsed();
            frame_index = frame_index.saturating_add(1);
            if let Some(rta) = rta_manager.as_mut() {
                let events = rta.tick(frame_index, now, |addr| core.read_memory(addr));
                rta.record_input_frame(
                    frame_index,
                    core.controller_bits(),
                    core.controller2_bits(),
                    now,
                );
                if events
                    .iter()
                    .any(|event| matches!(event, RtaEvent::Finished(_)))
                {
                    if let Err(err) = rta.write_artifacts_if_finished() {
                        eprintln!("RTA artifact write failed: {err}");
                    }
                    if let Some(rta_config) = runtime.rta.as_ref()
                        && rta.is_calibrating()
                        && let Err(err) = rta.write_calibration_draft(&rta_config.profiles_dir)
                    {
                        eprintln!("RTA calibration draft write failed: {err}");
                    }
                }
            }
            metrics.on_step(&core, step_elapsed, missed_deadline);
            if let Some(stats) = netplay_stats.as_ref() {
                metrics.on_netplay_stats(stats);
            }
            if should_trace_frame(trace_every_frames, frame_index) {
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
                metrics.on_audio_queue(audio_output.queue_len(), audio_queue_dropped(queued));
            }

            window.request_redraw();
            *control_flow = ControlFlow::WaitUntil(next_frame_deadline);
        }
        Event::RedrawRequested(_) => {
            let render_start = Instant::now();
            core.fill_framebuffer_rgba(&mut frame_rgba);
            pixels.frame_mut().copy_from_slice(&frame_rgba);
            if let Some(config) = capture.as_ref()
                && should_capture_frame(config.every_n_frames, frame_index)
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
    let netplay_enabled =
        netplay_feature_enabled(runtime_args.netplay_enabled, config.netplay.enabled);
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
    let rta_enabled = runtime_args.rta_enabled
        || runtime_args.rta_profile_id.is_some()
        || runtime_args.rta_profiles_dir.is_some()
        || runtime_args.rta_runs_dir.is_some()
        || runtime_args.rta_calibrate;
    let rta = if rta_enabled {
        Some(RtaRuntimeConfig {
            profile_id_override: runtime_args.rta_profile_id.clone(),
            profiles_dir: PathBuf::from(
                runtime_args
                    .rta_profiles_dir
                    .clone()
                    .unwrap_or_else(|| DEFAULT_RTA_PROFILES_DIR.to_owned()),
            ),
            runs_dir: PathBuf::from(
                runtime_args
                    .rta_runs_dir
                    .clone()
                    .unwrap_or_else(|| DEFAULT_RTA_RUNS_DIR.to_owned()),
            ),
            calibrate: runtime_args.rta_calibrate,
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
        rta,
        #[cfg(feature = "nova")]
        auto_player_enabled: runtime_args.auto_player_enabled,
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
        rta_enabled: false,
        rta_profile_id: None,
        rta_profiles_dir: None,
        rta_runs_dir: None,
        rta_calibrate: false,
        #[cfg(feature = "nova")]
        auto_player_enabled: false,
    };
    let mut idx = 0_usize;
    while idx < args.len() {
        let arg = &args[idx];
        if arg == "--help" || arg == "-h" {
            return Err(format!(
                "Usage: nes-desktop [--config <path>] [--mcp-host] [--mcp-bind <addr>] [--netplay] [--netplay-relay <addr>] [--netplay-room <room>] [--netplay-player <1|2>] [--netplay-delay <frames>] [--netplay-max-rollback <frames>] [--netplay-hash-every <frames>] [--rta] [--rta-profile <id>] [--rta-profiles-dir <path>] [--rta-runs-dir <path>] [--rta-calibrate] [rom_path]\nDefault config path: {DEFAULT_CONFIG_PATH}"
            ));
        }
        if arg == "--mcp-host" {
            parsed.mcp_enabled = true;
            idx += 1;
            continue;
        }
        #[cfg(feature = "nova")]
        if arg == "--auto-player" {
            parsed.auto_player_enabled = true;
            idx += 1;
            continue;
        }
        if arg == "--netplay" {
            parsed.netplay_enabled = true;
            idx += 1;
            continue;
        }
        if arg == "--rta" {
            parsed.rta_enabled = true;
            idx += 1;
            continue;
        }
        if arg == "--rta-calibrate" {
            parsed.rta_calibrate = true;
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
        if arg == "--rta-profile" {
            let Some(profile_id) = args.get(idx + 1) else {
                return Err("missing value after --rta-profile".to_owned());
            };
            parsed.rta_profile_id = Some(profile_id.clone());
            idx += 2;
            continue;
        }
        if arg == "--rta-profiles-dir" {
            let Some(path) = args.get(idx + 1) else {
                return Err("missing value after --rta-profiles-dir".to_owned());
            };
            parsed.rta_profiles_dir = Some(path.clone());
            idx += 2;
            continue;
        }
        if arg == "--rta-runs-dir" {
            let Some(path) = args.get(idx + 1) else {
                return Err("missing value after --rta-runs-dir".to_owned());
            };
            parsed.rta_runs_dir = Some(path.clone());
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
        if let Some(profile_id) = arg.strip_prefix("--rta-profile=") {
            if profile_id.is_empty() {
                return Err("missing value after --rta-profile=".to_owned());
            }
            parsed.rta_profile_id = Some(profile_id.to_owned());
            idx += 1;
            continue;
        }
        if let Some(path) = arg.strip_prefix("--rta-profiles-dir=") {
            if path.is_empty() {
                return Err("missing value after --rta-profiles-dir=".to_owned());
            }
            parsed.rta_profiles_dir = Some(path.to_owned());
            idx += 1;
            continue;
        }
        if let Some(path) = arg.strip_prefix("--rta-runs-dir=") {
            if path.is_empty() {
                return Err("missing value after --rta-runs-dir=".to_owned());
            }
            parsed.rta_runs_dir = Some(path.to_owned());
            idx += 1;
            continue;
        }
        if arg.starts_with("--") {
            return Err(format!(
                "unknown flag '{arg}'. Usage: nes-desktop [--config <path>] [--mcp-host] [--mcp-bind <addr>] [--netplay] [--netplay-relay <addr>] [--netplay-room <room>] [--netplay-player <1|2>] [--netplay-delay <frames>] [--netplay-max-rollback <frames>] [--netplay-hash-every <frames>] [--rta] [--rta-profile <id>] [--rta-profiles-dir <path>] [--rta-runs-dir <path>] [--rta-calibrate] [rom_path]"
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
        target.max(current_delay_frames.saturating_add(1))
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

#[derive(Debug, Clone, Copy, Default)]
struct GamepadSnapshot {
    connected: bool,
    south_pressed: bool,
    east_pressed: bool,
    west_pressed: bool,
    north_pressed: bool,
    select_pressed: bool,
    start_pressed: bool,
    dpad_up_pressed: bool,
    dpad_down_pressed: bool,
    dpad_left_pressed: bool,
    dpad_right_pressed: bool,
    left_x: f32,
    left_y: f32,
}

fn connected_gamepad_ids(gamepads: impl IntoIterator<Item = (GamepadId, bool)>) -> Vec<GamepadId> {
    gamepads
        .into_iter()
        .filter_map(|(id, connected)| connected.then_some(id))
        .collect()
}

fn select_active_gamepad_ids(
    connected: &[GamepadId],
    current: [Option<GamepadId>; 2],
) -> [Option<GamepadId>; 2] {
    let mut next = [None::<GamepadId>; 2];

    for player in 0..next.len() {
        if let Some(gamepad_id) = current[player]
            && connected.contains(&gamepad_id)
            && !next.contains(&Some(gamepad_id))
        {
            next[player] = Some(gamepad_id);
        }
    }

    for &gamepad_id in connected {
        if next.iter().all(|slot| *slot != Some(gamepad_id))
            && let Some(slot) = next.iter_mut().find(|slot| slot.is_none())
        {
            *slot = Some(gamepad_id);
        }
    }

    next
}

fn gamepad_snapshot_to_bits(snapshot: GamepadSnapshot) -> u8 {
    if !snapshot.connected {
        return 0;
    }

    let mut bits = 0_u8;
    // Keep both common face layouts usable across Xbox/Switch-style controllers.
    if snapshot.south_pressed || snapshot.east_pressed {
        bits |= Button::A.bit_mask();
    }
    if snapshot.west_pressed || snapshot.north_pressed {
        bits |= Button::B.bit_mask();
    }
    if snapshot.select_pressed {
        bits |= Button::Select.bit_mask();
    }
    if snapshot.start_pressed {
        bits |= Button::Start.bit_mask();
    }

    if snapshot.dpad_up_pressed || snapshot.left_y <= -GAMEPAD_AXIS_THRESHOLD {
        bits |= Button::Up.bit_mask();
    }
    if snapshot.dpad_down_pressed || snapshot.left_y >= GAMEPAD_AXIS_THRESHOLD {
        bits |= Button::Down.bit_mask();
    }
    if snapshot.dpad_left_pressed || snapshot.left_x <= -GAMEPAD_AXIS_THRESHOLD {
        bits |= Button::Left.bit_mask();
    }
    if snapshot.dpad_right_pressed || snapshot.left_x >= GAMEPAD_AXIS_THRESHOLD {
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
        nes_core::bmp::encode_bmp(FRAME_WIDTH, FRAME_HEIGHT, rgba)?
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

fn format_rom_read_error(rom_path: &str, err: &std::io::Error) -> String {
    if err.kind() == std::io::ErrorKind::NotFound {
        format!(
            "Error: Could not find the ROM file at '{}'.\nHint: Check the path or try the bundled homebrew ROM: ./roms/homebrew/homebrew.nes",
            rom_path
        )
    } else {
        format!("Error: Failed to read ROM at '{}': {}", rom_path, err)
    }
}

#[cfg(test)]
mod tests {
    use super::{
        AudioOutput, AudioSinkControl, DEFAULT_CAPTURE_EVERY_FRAMES, DEFAULT_MCP_BIND_ADDR,
        DEFAULT_METRICS_EVERY_FRAMES, FRAME_HEIGHT, FRAME_WIDTH, FrameDecision,
        GAMEPAD_AXIS_THRESHOLD, GamepadSnapshot, KeyboardContext, KeyboardDecision,
        MAX_AUDIO_QUEUE_CHUNKS, MetricsSnapshot, NetplayRuntimeStats, PerfMetrics,
        RodioSinkAdapter, RuntimeArgs, StepMode, TARGET_FRAME_TIME, WindowEventDecision,
        advance_core_for_host_frame, apply_gamepad_delta_commands, audio_queue_dropped,
        capture_config_from_parts, capture_path_for_frame, classify_keyboard_input,
        classify_window_event, compute_local_netplay_bits, compute_metrics_snapshot,
        connected_gamepad_ids, controller_state_delta_for_player, element_state_pressed,
        encode_ppm, evaluate_frame_deadline, format_rom_read_error, frame_signature,
        gamepad_assignments_changed, gamepad_slot_changed, gamepad_snapshot_to_bits,
        handle_netplay_server_message, is_player_two_slot, map_virtual_keycode,
        merge_local_input_bits, netplay_feature_enabled, parse_runtime_args,
        recommended_input_delay_frames, resync_restored_inputs, scaled_window_dimensions,
        schedule_netplay_ping, select_active_gamepad_ids, should_capture_frame,
        should_log_rollback, should_resume_after_rewind_hold, should_send_netplay_hash,
        should_trace_frame, should_update_input_delay, track_keyboard_bits_for_key,
        update_button_bits, write_frame_ppm,
    };
    use gilrs::GamepadId;
    use nes_core::{Button, Command, NesCore};
    use nes_netplay::{RollbackConfig, RollbackEngine, ServerMessage};
    use rodio::Sink;
    use std::collections::BTreeMap;
    use std::fs;
    use std::sync::mpsc;
    use std::sync::{Arc, Mutex};
    use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
    use winit::dpi::PhysicalSize;
    use winit::event::{
        DeviceId, ElementState, KeyboardInput, ModifiersState, VirtualKeyCode, WindowEvent,
    };

    fn parse_runtime_args_with_timeout(args: Vec<String>) -> Result<RuntimeArgs, String> {
        let (tx, rx) = mpsc::channel();
        std::thread::spawn(move || {
            let _ = tx.send(parse_runtime_args(&args));
        });
        rx.recv_timeout(Duration::from_millis(250))
            .expect("parse_runtime_args should complete without hanging")
    }

    fn sample_ines(mapper_id: u8, prg_banks: u8) -> Vec<u8> {
        let mut rom = vec![0_u8; 16 + prg_banks as usize * 16 * 1024];
        rom[0] = 0x4E; // N
        rom[1] = 0x45; // E
        rom[2] = 0x53; // S
        rom[3] = 0x1A;
        rom[4] = prg_banks;
        rom[5] = 0; // CHR RAM
        rom[6] = (mapper_id & 0x0F) << 4;
        rom[7] = mapper_id & 0xF0;
        rom
    }

    fn fake_gamepad_id(raw: usize) -> GamepadId {
        // SAFETY: Test-only helper to synthesize opaque identifiers for equality checks.
        unsafe { std::mem::transmute::<usize, GamepadId>(raw) }
    }

    #[derive(Debug, Default)]
    struct FakeAudioState {
        queue_len: usize,
        append_calls: usize,
        appended_sample_count: usize,
        clear_calls: usize,
        stop_calls: usize,
    }

    struct FakeAudioSink {
        state: Arc<Mutex<FakeAudioState>>,
    }

    impl FakeAudioSink {
        fn with_len(initial_len: usize) -> (Self, Arc<Mutex<FakeAudioState>>) {
            let state = Arc::new(Mutex::new(FakeAudioState {
                queue_len: initial_len,
                ..Default::default()
            }));
            (
                Self {
                    state: Arc::clone(&state),
                },
                state,
            )
        }
    }

    impl AudioSinkControl for FakeAudioSink {
        fn queue_len(&self) -> usize {
            self.state
                .lock()
                .expect("fake audio state lock should not poison")
                .queue_len
        }

        fn append_i16(&self, samples: Vec<i16>) {
            let mut state = self
                .state
                .lock()
                .expect("fake audio state lock should not poison");
            state.append_calls = state.append_calls.saturating_add(1);
            state.appended_sample_count = state.appended_sample_count.saturating_add(samples.len());
            state.queue_len = state.queue_len.saturating_add(1);
        }

        fn stop(&self) {
            let mut state = self
                .state
                .lock()
                .expect("fake audio state lock should not poison");
            state.stop_calls = state.stop_calls.saturating_add(1);
        }

        fn clear(&self) {
            let mut state = self
                .state
                .lock()
                .expect("fake audio state lock should not poison");
            state.clear_calls = state.clear_calls.saturating_add(1);
            state.queue_len = 0;
        }
    }

    #[test]
    fn parse_runtime_args_accepts_mcp_host_and_bind_flags() {
        let args = vec![
            "--mcp-host".to_owned(),
            "--mcp-bind".to_owned(),
            "127.0.0.1:7777".to_owned(),
            "game.nes".to_owned(),
        ];
        let parsed = parse_runtime_args_with_timeout(args).expect("parse args");
        assert_eq!(parsed.rom_path.as_deref(), Some("game.nes"));
        assert!(parsed.mcp_enabled);
        assert_eq!(parsed.mcp_bind_addr, "127.0.0.1:7777");
    }

    #[test]
    fn parse_runtime_args_accepts_equals_bind_form() {
        let args = vec!["--mcp-bind=127.0.0.1:7000".to_owned()];
        let parsed = parse_runtime_args_with_timeout(args).expect("parse args");
        assert!(parsed.rom_path.is_none());
        assert!(!parsed.mcp_enabled);
        assert_eq!(parsed.mcp_bind_addr, "127.0.0.1:7000");
    }

    #[test]
    fn parse_runtime_args_defaults_bind_when_flag_absent() {
        let args = vec!["game.nes".to_owned()];
        let parsed = parse_runtime_args_with_timeout(args).expect("parse args");
        assert_eq!(parsed.rom_path.as_deref(), Some("game.nes"));
        assert!(!parsed.mcp_enabled);
        assert_eq!(parsed.mcp_bind_addr, DEFAULT_MCP_BIND_ADDR);
    }

    #[test]
    fn parse_runtime_args_rejects_unknown_flags() {
        let args = vec!["--bogus".to_owned()];
        let err = parse_runtime_args_with_timeout(args).expect_err("unknown flag should fail");
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
        let parsed = parse_runtime_args_with_timeout(args).expect("parse args");
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
            rta_enabled: false,
            rta_profile_id: None,
            rta_profiles_dir: None,
            rta_runs_dir: None,
            rta_calibrate: false,
            #[cfg(feature = "nova")]
            auto_player_enabled: false,
        };
        assert_eq!(parsed, expected);
    }

    #[test]
    fn parse_runtime_args_accepts_rta_flags() {
        let args = vec![
            "--rta".to_owned(),
            "--rta-profile=smb-any".to_owned(),
            "--rta-profiles-dir".to_owned(),
            "config/rta/profiles".to_owned(),
            "--rta-runs-dir=runs/rta".to_owned(),
            "--rta-calibrate".to_owned(),
        ];
        let parsed = parse_runtime_args_with_timeout(args).expect("parse args");

        assert!(parsed.rta_enabled);
        assert_eq!(parsed.rta_profile_id.as_deref(), Some("smb-any"));
        assert_eq!(
            parsed.rta_profiles_dir.as_deref(),
            Some("config/rta/profiles")
        );
        assert_eq!(parsed.rta_runs_dir.as_deref(), Some("runs/rta"));
        assert!(parsed.rta_calibrate);
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
    fn adaptive_delay_exact_targets_and_hysteresis_behave_as_expected() {
        let raised = recommended_input_delay_frames(Some(96.0), 12.0, 1, 12, 2);
        assert_eq!(raised, 5);

        let clamped = recommended_input_delay_frames(Some(400.0), 100.0, 1, 6, 2);
        assert_eq!(clamped, 6);

        let drops_by_one = recommended_input_delay_frames(Some(40.0), 0.0, 1, 12, 6);
        assert_eq!(drops_by_one, 5);

        let holds_within_hysteresis = recommended_input_delay_frames(Some(40.0), 0.0, 1, 12, 4);
        assert_eq!(holds_within_hysteresis, 4);

        let jitter_weighted = recommended_input_delay_frames(Some(1.0), 12.0, 1, 12, 1);
        assert_eq!(jitter_weighted, 3);

        let equal_target_holds_current = recommended_input_delay_frames(Some(1.0), 12.0, 1, 12, 3);
        assert_eq!(equal_target_holds_current, 3);
    }

    #[test]
    fn adaptive_delay_returns_min_when_bounds_are_invalid() {
        assert_eq!(recommended_input_delay_frames(Some(80.0), 8.0, 5, 5, 2), 5);
        assert_eq!(recommended_input_delay_frames(Some(80.0), 8.0, 6, 4, 2), 6);
    }

    #[test]
    fn compute_metrics_snapshot_derives_expected_rates() {
        let snapshot = compute_metrics_snapshot(
            2,
            0.25,
            100,
            130,
            Duration::from_millis(10),
            Duration::from_millis(8),
        )
        .expect("non-zero frames and elapsed should yield snapshot");
        assert_eq!(
            snapshot,
            MetricsSnapshot {
                wall_fps: 8.0,
                emu_fps: 120.0,
                avg_step_ms: 5.0,
                avg_render_ms: 4.0,
            }
        );
    }

    #[test]
    fn compute_metrics_snapshot_handles_guard_conditions_and_saturating_ppu_delta() {
        assert!(compute_metrics_snapshot(0, 1.0, 10, 20, Duration::ZERO, Duration::ZERO).is_none());
        assert!(
            compute_metrics_snapshot(1, f64::EPSILON, 10, 20, Duration::ZERO, Duration::ZERO)
                .is_none()
        );
        let snapshot = compute_metrics_snapshot(
            1,
            1.0,
            200,
            100,
            Duration::from_millis(3),
            Duration::from_millis(2),
        )
        .expect("valid input should produce snapshot");
        assert_eq!(snapshot.emu_fps, 0.0);
    }

    #[test]
    fn audio_output_queue_and_drop_behave_with_fake_sink() {
        let (fake_sink, shared_state) = FakeAudioSink::with_len(MAX_AUDIO_QUEUE_CHUNKS - 1);
        let output = AudioOutput::new_with_sink(Box::new(fake_sink), None);

        assert!(output.queue_samples(vec![1_i16, 2, 3]));
        assert_eq!(output.queue_len(), MAX_AUDIO_QUEUE_CHUNKS);

        let state_after_queue = shared_state
            .lock()
            .expect("fake audio state lock should not poison");
        assert_eq!(state_after_queue.append_calls, 1);
        assert_eq!(state_after_queue.appended_sample_count, 3);
        drop(state_after_queue);

        drop(output);
        let state_after_drop = shared_state
            .lock()
            .expect("fake audio state lock should not poison");
        assert_eq!(state_after_drop.stop_calls, 1);
    }

    #[test]
    fn audio_output_rejects_samples_when_queue_is_full() {
        let (fake_sink, shared_state) = FakeAudioSink::with_len(MAX_AUDIO_QUEUE_CHUNKS);
        let output = AudioOutput::new_with_sink(Box::new(fake_sink), None);

        assert!(!output.queue_samples(vec![1_i16, 2, 3]));
        assert_eq!(output.queue_len(), MAX_AUDIO_QUEUE_CHUNKS);

        let state = shared_state
            .lock()
            .expect("fake audio state lock should not poison");
        assert_eq!(state.append_calls, 0);
    }

    #[test]
    fn audio_output_clear_forwards_to_sink_and_empties_queue() {
        let (fake_sink, shared_state) = FakeAudioSink::with_len(3);
        let output = AudioOutput::new_with_sink(Box::new(fake_sink), None);

        output.clear();

        let state = shared_state
            .lock()
            .expect("fake audio state lock should not poison");
        assert_eq!(state.clear_calls, 1);
        assert_eq!(state.queue_len, 0);
    }

    #[test]
    fn rodio_sink_adapter_forwards_append_and_queue_len() {
        let (sink, _queue_rx) = Sink::new_idle();
        let adapter = RodioSinkAdapter { inner: sink };

        assert_eq!(adapter.queue_len(), 0);
        adapter.append_i16(vec![1_i16, 2, 3, 4]);
        assert_eq!(adapter.queue_len(), 1);
        adapter.append_i16(vec![5_i16, 6, 7, 8]);
        assert_eq!(adapter.queue_len(), 2);
    }

    #[test]
    fn rodio_sink_adapter_stop_mutes_idle_queue_output() {
        let (sink, mut queue_rx) = Sink::new_idle();
        let adapter = RodioSinkAdapter { inner: sink };

        adapter.append_i16(vec![i16::MAX; 5_000]);
        assert_eq!(
            queue_rx.next(),
            Some(0.999_969_5),
            "i16::MAX should map to a non-zero sample before stop"
        );

        adapter.stop();

        let saw_silence = (0..600).any(|_| queue_rx.next() == Some(0.0));
        assert!(saw_silence, "stop should silence the queued source");
    }

    #[test]
    fn map_virtual_keycode_maps_all_supported_keys() {
        assert_eq!(map_virtual_keycode(VirtualKeyCode::Z), Some("KeyZ"));
        assert_eq!(map_virtual_keycode(VirtualKeyCode::X), Some("KeyX"));
        assert_eq!(map_virtual_keycode(VirtualKeyCode::Return), Some("Enter"));
        assert_eq!(
            map_virtual_keycode(VirtualKeyCode::RShift),
            Some("ShiftRight")
        );
        assert_eq!(map_virtual_keycode(VirtualKeyCode::Up), Some("ArrowUp"));
        assert_eq!(map_virtual_keycode(VirtualKeyCode::Down), Some("ArrowDown"));
        assert_eq!(map_virtual_keycode(VirtualKeyCode::Left), Some("ArrowLeft"));
        assert_eq!(
            map_virtual_keycode(VirtualKeyCode::Right),
            Some("ArrowRight")
        );
        assert_eq!(map_virtual_keycode(VirtualKeyCode::Escape), None);
    }

    #[test]
    #[allow(deprecated)]
    fn classify_window_event_maps_window_variants_to_decisions() {
        assert_eq!(
            classify_window_event(&WindowEvent::CloseRequested),
            WindowEventDecision::CloseRequested
        );

        let key_event = WindowEvent::KeyboardInput {
            // SAFETY: winit explicitly exposes dummy IDs for unit testing.
            device_id: unsafe { DeviceId::dummy() },
            input: KeyboardInput {
                scancode: 0,
                state: ElementState::Pressed,
                virtual_keycode: Some(VirtualKeyCode::Z),
                modifiers: ModifiersState::empty(),
            },
            is_synthetic: false,
        };
        assert_eq!(
            classify_window_event(&key_event),
            WindowEventDecision::KeyboardInput {
                key: Some(VirtualKeyCode::Z),
                pressed: true
            }
        );

        let resized = WindowEvent::Resized(PhysicalSize::new(640, 480));
        assert_eq!(
            classify_window_event(&resized),
            WindowEventDecision::Resized {
                width: 640,
                height: 480
            }
        );

        let mut scale_size = PhysicalSize::new(800, 600);
        let scale_changed = WindowEvent::ScaleFactorChanged {
            scale_factor: 1.25,
            new_inner_size: &mut scale_size,
        };
        assert_eq!(
            classify_window_event(&scale_changed),
            WindowEventDecision::ScaleFactorChanged {
                width: 800,
                height: 600
            }
        );

        let ignored = WindowEvent::Focused(true);
        assert_eq!(classify_window_event(&ignored), WindowEventDecision::Ignore);
    }

    #[test]
    fn classify_keyboard_input_covers_exit_rewind_rollback_and_core_paths() {
        assert_eq!(
            classify_keyboard_input(
                VirtualKeyCode::Escape,
                true,
                KeyboardContext {
                    rollback_enabled: false,
                    rta_enabled: false,
                    rta_calibrate: false,
                }
            ),
            KeyboardDecision::Exit
        );
        assert_eq!(
            classify_keyboard_input(
                VirtualKeyCode::F5,
                true,
                KeyboardContext {
                    rollback_enabled: false,
                    rta_enabled: false,
                    rta_calibrate: false,
                }
            ),
            KeyboardDecision::ManualSaveState
        );
        assert_eq!(
            classify_keyboard_input(
                VirtualKeyCode::F8,
                true,
                KeyboardContext {
                    rollback_enabled: false,
                    rta_enabled: false,
                    rta_calibrate: false,
                }
            ),
            KeyboardDecision::ManualLoadState
        );
        assert_eq!(
            classify_keyboard_input(
                VirtualKeyCode::R,
                true,
                KeyboardContext {
                    rollback_enabled: false,
                    rta_enabled: false,
                    rta_calibrate: false,
                }
            ),
            KeyboardDecision::SetRewindHeld(true)
        );
        assert_eq!(
            classify_keyboard_input(
                VirtualKeyCode::R,
                false,
                KeyboardContext {
                    rollback_enabled: true,
                    rta_enabled: false,
                    rta_calibrate: false,
                }
            ),
            KeyboardDecision::SetRewindHeld(false)
        );
        assert_eq!(
            classify_keyboard_input(
                VirtualKeyCode::Z,
                true,
                KeyboardContext {
                    rollback_enabled: true,
                    rta_enabled: false,
                    rta_calibrate: false,
                }
            ),
            KeyboardDecision::UpdateKeyboardBits {
                mask: Button::A.bit_mask(),
                pressed: true
            }
        );
        assert_eq!(
            classify_keyboard_input(
                VirtualKeyCode::X,
                false,
                KeyboardContext {
                    rollback_enabled: false,
                    rta_enabled: false,
                    rta_calibrate: false,
                }
            ),
            KeyboardDecision::ExecuteCore(Command::ReleaseButton(Button::B))
        );
        assert_eq!(
            classify_keyboard_input(
                VirtualKeyCode::Escape,
                false,
                KeyboardContext {
                    rollback_enabled: false,
                    rta_enabled: false,
                    rta_calibrate: false,
                }
            ),
            KeyboardDecision::Noop
        );
        assert_eq!(
            classify_keyboard_input(
                VirtualKeyCode::F9,
                true,
                KeyboardContext {
                    rollback_enabled: false,
                    rta_enabled: true,
                    rta_calibrate: false,
                }
            ),
            KeyboardDecision::RtaManualSplit
        );
        assert_eq!(
            classify_keyboard_input(
                VirtualKeyCode::F10,
                true,
                KeyboardContext {
                    rollback_enabled: false,
                    rta_enabled: true,
                    rta_calibrate: true,
                }
            ),
            KeyboardDecision::RtaFinish
        );
        assert_eq!(
            classify_keyboard_input(
                VirtualKeyCode::F5,
                false,
                KeyboardContext {
                    rollback_enabled: false,
                    rta_enabled: false,
                    rta_calibrate: false,
                }
            ),
            KeyboardDecision::Noop
        );
    }

    #[test]
    fn evaluate_frame_deadline_classifies_wait_and_step_cases() {
        let now = Instant::now();
        let future = now + Duration::from_millis(2);
        match evaluate_frame_deadline(now, future) {
            FrameDecision::WaitUntil(deadline) => assert_eq!(deadline, future),
            FrameDecision::Step { .. } => panic!("expected wait branch"),
        }

        match evaluate_frame_deadline(now, now) {
            FrameDecision::Step {
                missed_deadline,
                next_deadline,
            } => {
                assert!(!missed_deadline);
                assert_eq!(next_deadline, now + TARGET_FRAME_TIME);
            }
            FrameDecision::WaitUntil(_) => panic!("expected step branch"),
        }

        match evaluate_frame_deadline(now + Duration::from_millis(1), now) {
            FrameDecision::Step {
                missed_deadline,
                next_deadline,
            } => {
                assert!(missed_deadline);
                assert_eq!(
                    next_deadline,
                    (now + Duration::from_millis(1)) + TARGET_FRAME_TIME
                );
            }
            FrameDecision::WaitUntil(_) => panic!("expected step branch"),
        }
    }

    #[test]
    fn netplay_helper_functions_choose_local_bits_and_hash_schedule() {
        assert_eq!(compute_local_netplay_bits([0x12, 0x34], 1), 0x12);
        assert_eq!(compute_local_netplay_bits([0x12, 0x34], 2), 0x34);
        assert_eq!(compute_local_netplay_bits([0x12, 0x34], 3), 0x12);
        assert_eq!(compute_local_netplay_bits([0, 0], 9), 0);

        assert!(!should_send_netplay_hash(0, 120));
        assert!(!should_send_netplay_hash(60, 0));
        assert!(should_send_netplay_hash(60, 120));
        assert!(!should_send_netplay_hash(60, 121));
    }

    #[test]
    fn desktop_loop_helper_primitives_cover_window_scale_and_player_flags() {
        assert_eq!(
            scaled_window_dimensions(1),
            (FRAME_WIDTH as f64, FRAME_HEIGHT as f64)
        );
        assert_eq!(
            scaled_window_dimensions(3),
            (
                f64::from(FRAME_WIDTH as u32 * 3),
                f64::from(FRAME_HEIGHT as u32 * 3)
            )
        );

        assert!(element_state_pressed(ElementState::Pressed));
        assert!(!element_state_pressed(ElementState::Released));
        assert!(should_resume_after_rewind_hold(false));
        assert!(!should_resume_after_rewind_hold(true));

        assert!(!is_player_two_slot(0));
        assert!(is_player_two_slot(1));
        assert_eq!(
            merge_local_input_bits(0b0000_0011, 0b0000_0101),
            0b0000_0111
        );

        assert!(netplay_feature_enabled(true, false));
        assert!(netplay_feature_enabled(false, true));
        assert!(!netplay_feature_enabled(false, false));

        assert!(!should_log_rollback(0));
        assert!(should_log_rollback(1));
        assert!(!should_update_input_delay(2, 2));
        assert!(should_update_input_delay(3, 2));

        assert!(!should_trace_frame(0, 120));
        assert!(!should_trace_frame(60, 0));
        assert!(should_trace_frame(60, 120));
        assert!(!should_trace_frame(60, 121));

        assert!(!audio_queue_dropped(true));
        assert!(audio_queue_dropped(false));

        assert!(!should_capture_frame(0, 120));
        assert!(should_capture_frame(60, 120));
        assert!(!should_capture_frame(60, 121));
    }

    #[test]
    fn gamepad_assignment_helpers_detect_global_and_slot_level_changes() {
        let none = [None, None];
        assert!(!gamepad_assignments_changed(none, none));
        assert!(!gamepad_slot_changed(none, none, 0));
        assert!(!gamepad_slot_changed(none, none, 1));

        let next = [Some(fake_gamepad_id(1)), None];
        let current = [None, Some(fake_gamepad_id(2))];
        assert!(gamepad_assignments_changed(next, current));
        assert!(gamepad_slot_changed(next, current, 0));
        assert!(gamepad_slot_changed(next, current, 1));
    }

    #[test]
    fn gamepad_source_helpers_select_connected_ids_without_duplicates() {
        let id1 = fake_gamepad_id(1);
        let id2 = fake_gamepad_id(2);
        let id3 = fake_gamepad_id(3);
        let connected = connected_gamepad_ids(vec![(id1, true), (id2, false), (id3, true)]);
        assert_eq!(connected, vec![id1, id3]);

        let next = select_active_gamepad_ids(&connected, [Some(id1), Some(id2)]);
        assert_eq!(next, [Some(id1), Some(id3)]);

        let deduped = select_active_gamepad_ids(&connected, [Some(id3), Some(id3)]);
        assert_eq!(deduped, [Some(id3), Some(id1)]);
    }

    #[test]
    fn gamepad_sampling_helpers_map_buttons_and_axis_thresholds() {
        let bits = gamepad_snapshot_to_bits(GamepadSnapshot {
            connected: true,
            east_pressed: true,
            north_pressed: true,
            select_pressed: true,
            start_pressed: true,
            dpad_down_pressed: true,
            dpad_right_pressed: true,
            left_x: -0.75,
            left_y: -0.75,
            ..GamepadSnapshot::default()
        });
        let expected = Button::A.bit_mask()
            | Button::B.bit_mask()
            | Button::Select.bit_mask()
            | Button::Start.bit_mask()
            | Button::Up.bit_mask()
            | Button::Down.bit_mask()
            | Button::Left.bit_mask()
            | Button::Right.bit_mask();
        assert_eq!(bits, expected);

        let boundary_bits = gamepad_snapshot_to_bits(GamepadSnapshot {
            connected: true,
            left_x: GAMEPAD_AXIS_THRESHOLD,
            left_y: -GAMEPAD_AXIS_THRESHOLD,
            ..GamepadSnapshot::default()
        });
        assert_eq!(
            boundary_bits,
            Button::Up.bit_mask() | Button::Right.bit_mask()
        );

        let neutral_axis_bits = gamepad_snapshot_to_bits(GamepadSnapshot {
            connected: true,
            left_x: 0.0,
            left_y: GAMEPAD_AXIS_THRESHOLD * 0.5,
            ..GamepadSnapshot::default()
        });
        assert_eq!(neutral_axis_bits, 0);

        assert_eq!(
            gamepad_snapshot_to_bits(GamepadSnapshot {
                connected: false,
                east_pressed: true,
                left_x: 1.0,
                left_y: -1.0,
                ..GamepadSnapshot::default()
            }),
            0
        );
    }

    #[test]
    fn schedule_netplay_ping_enforces_deadline_nonce_and_pending_cap() {
        let now = Instant::now();
        let mut next_ping_at = now + Duration::from_millis(10);
        let mut nonce = 5_u64;
        let mut pending = BTreeMap::<u64, Instant>::new();
        pending.insert(1, now - Duration::from_millis(20));
        pending.insert(2, now - Duration::from_millis(10));

        assert_eq!(
            schedule_netplay_ping(
                now,
                &mut next_ping_at,
                &mut nonce,
                &mut pending,
                Duration::from_millis(500),
                2
            ),
            None
        );
        assert_eq!(nonce, 5);
        assert_eq!(pending.len(), 2);

        let due_now = now + Duration::from_millis(10);
        let scheduled = schedule_netplay_ping(
            due_now,
            &mut next_ping_at,
            &mut nonce,
            &mut pending,
            Duration::from_millis(500),
            2,
        );
        assert_eq!(scheduled, Some(5));
        assert_eq!(nonce, 6);
        assert_eq!(next_ping_at, due_now + Duration::from_millis(500));
        assert_eq!(pending.len(), 2, "pending set should be capped to max");
        assert!(
            !pending.contains_key(&1),
            "oldest nonce should be evicted first"
        );
        assert!(pending.contains_key(&5), "new nonce should be tracked");
    }

    #[test]
    fn update_button_bits_sets_and_clears_masks() {
        let with_a = update_button_bits(0, Button::A.bit_mask(), true);
        assert_eq!(with_a, Button::A.bit_mask());
        // Pressing an already-set bit should be idempotent.
        assert_eq!(
            update_button_bits(with_a, Button::A.bit_mask(), true),
            Button::A.bit_mask()
        );
        let with_ab = update_button_bits(with_a, Button::B.bit_mask(), true);
        assert_eq!(with_ab, Button::A.bit_mask() | Button::B.bit_mask());
        let cleared_a = update_button_bits(with_ab, Button::A.bit_mask(), false);
        assert_eq!(cleared_a, Button::B.bit_mask());
    }

    #[test]
    fn track_keyboard_bits_for_key_updates_controller_bits_and_ignores_hotkeys() {
        let mut keyboard_bits = 0_u8;

        track_keyboard_bits_for_key(VirtualKeyCode::Z, true, &mut keyboard_bits);
        assert_eq!(keyboard_bits, Button::A.bit_mask());

        track_keyboard_bits_for_key(VirtualKeyCode::F5, true, &mut keyboard_bits);
        assert_eq!(
            keyboard_bits,
            Button::A.bit_mask(),
            "manual save hotkey must not alter controller state"
        );

        track_keyboard_bits_for_key(VirtualKeyCode::Z, false, &mut keyboard_bits);
        assert_eq!(keyboard_bits, 0);
    }

    #[test]
    fn resync_restored_inputs_reapplies_keyboard_and_resets_gamepad_cache() {
        let mut core = NesCore::new();
        let mut gamepad_bits = [Button::Right.bit_mask(), Button::Start.bit_mask()];

        resync_restored_inputs(&mut core, Button::A.bit_mask(), &mut gamepad_bits)
            .expect("restored inputs should resync");

        assert_eq!(
            core.controller_bits(),
            Button::A.bit_mask(),
            "held keyboard input should be re-applied immediately"
        );
        assert_eq!(
            core.controller2_bits(),
            0,
            "player-2 gamepad state should be cleared until the next poll replays it"
        );
        assert_eq!(
            gamepad_bits,
            [0, 0],
            "gamepad cache must reset so held pads generate deltas on the next poll"
        );
    }

    #[test]
    fn apply_gamepad_delta_commands_updates_controller_bits() {
        let mut core = NesCore::new();
        apply_gamepad_delta_commands(
            &mut core,
            0,
            Button::A.bit_mask() | Button::Right.bit_mask(),
            false,
        )
        .expect("applying player-1 gamepad delta should succeed");
        assert_eq!(
            core.controller_bits(),
            Button::A.bit_mask() | Button::Right.bit_mask()
        );

        apply_gamepad_delta_commands(
            &mut core,
            Button::A.bit_mask() | Button::Right.bit_mask(),
            Button::Right.bit_mask(),
            false,
        )
        .expect("releasing one player-1 button should succeed");
        assert_eq!(core.controller_bits(), Button::Right.bit_mask());

        apply_gamepad_delta_commands(&mut core, 0, Button::Start.bit_mask(), true)
            .expect("applying player-2 gamepad delta should succeed");
        assert_eq!(core.controller2_bits(), Button::Start.bit_mask());
    }

    #[test]
    fn handle_netplay_server_message_updates_stats_and_errors() {
        let mut core = NesCore::new();
        let mut rom = sample_ines(0, 1);
        let prg_start = 16;
        rom[prg_start + 0x3FFC] = 0x00;
        rom[prg_start + 0x3FFD] = 0x80;
        core.load_ines_rom(&rom).expect("sample rom should load");

        let mut rollback_engine = RollbackEngine::new(RollbackConfig {
            local_player: 1,
            input_delay_frames: 2,
            max_rollback_frames: 16,
        })
        .expect("rollback config should be valid");
        let first_step = rollback_engine
            .advance_frame(&mut core)
            .expect("initial rollback step should succeed");

        let mut stats = Some(NetplayRuntimeStats::new(2));
        let mut pending = BTreeMap::<u64, Instant>::new();
        pending.insert(7, Instant::now() - Duration::from_millis(10));

        handle_netplay_server_message(
            ServerMessage::Pong { nonce: 7 },
            &mut rollback_engine,
            1,
            &mut stats,
            &mut pending,
        )
        .expect("pong message should process");
        assert!(pending.is_empty());
        let rtt_ms = stats.as_ref().and_then(|s| s.latest_rtt_ms).unwrap_or(0.0);
        assert!(
            (1.0..=500.0).contains(&rtt_ms),
            "expected plausible RTT ms value, got {rtt_ms}"
        );

        handle_netplay_server_message(
            ServerMessage::PeerInput {
                player: 2,
                frame: first_step.frame,
                bits: 0x01,
            },
            &mut rollback_engine,
            1,
            &mut stats,
            &mut pending,
        )
        .expect("peer input should process");
        let rollback_step = rollback_engine
            .advance_frame(&mut core)
            .expect("post-input rollback step should succeed");
        assert!(
            rollback_step.rollback_distance > 0,
            "late remote input should queue rollback"
        );

        handle_netplay_server_message(
            ServerMessage::PeerHash {
                player: 2,
                frame: rollback_step.frame,
                state_hash: rollback_step.state_hash.wrapping_add(1),
            },
            &mut rollback_engine,
            1,
            &mut stats,
            &mut pending,
        )
        .expect("peer hash should process");
        assert_eq!(stats.as_ref().map_or(0, |s| s.desync_count), 1);

        let err = handle_netplay_server_message(
            ServerMessage::Error {
                message: "boom".to_owned(),
            },
            &mut rollback_engine,
            1,
            &mut stats,
            &mut pending,
        )
        .expect_err("relay errors should propagate");
        assert!(err.contains("relay error"));
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

    #[test]
    fn parse_runtime_args_help_and_validation_paths() {
        let help = parse_runtime_args_with_timeout(vec!["--help".to_owned()])
            .expect_err("help returns usage");
        assert!(help.contains("Usage: nes-desktop"));
        assert!(help.contains("Default config path"));

        let missing_bind = parse_runtime_args_with_timeout(vec!["--mcp-bind".to_owned()])
            .expect_err("missing mcp bind");
        assert!(missing_bind.contains("missing value after --mcp-bind"));

        let missing_relay = parse_runtime_args_with_timeout(vec!["--netplay-relay".to_owned()])
            .expect_err("missing netplay relay");
        assert!(missing_relay.contains("missing value after --netplay-relay"));

        let invalid_player =
            parse_runtime_args_with_timeout(vec!["--netplay-player".to_owned(), "bad".to_owned()])
                .expect_err("invalid player value");
        assert!(invalid_player.contains("--netplay-player"));

        let multi_rom =
            parse_runtime_args_with_timeout(vec!["a.nes".to_owned(), "b.nes".to_owned()])
                .expect_err("multiple roms should fail");
        assert!(multi_rom.contains("multiple ROM paths"));
    }

    #[test]
    fn parse_runtime_args_accepts_all_equals_forms_for_netplay_flags() {
        let args = vec![
            "--mcp-host".to_owned(),
            "--mcp-bind=127.0.0.1:7777".to_owned(),
            "--netplay".to_owned(),
            "--netplay-relay=relay.example:4545".to_owned(),
            "--netplay-room=arena".to_owned(),
            "--netplay-player=2".to_owned(),
            "--netplay-delay=3".to_owned(),
            "--netplay-max-rollback=360".to_owned(),
            "--netplay-hash-every=90".to_owned(),
            "rom.nes".to_owned(),
        ];
        let parsed = parse_runtime_args_with_timeout(args).expect("equals forms should parse");
        assert_eq!(parsed.rom_path.as_deref(), Some("rom.nes"));
        assert!(parsed.mcp_enabled);
        assert_eq!(parsed.mcp_bind_addr, "127.0.0.1:7777");
        assert!(parsed.netplay_enabled);
        assert_eq!(
            parsed.netplay_relay_addr.as_deref(),
            Some("relay.example:4545")
        );
        assert_eq!(parsed.netplay_room.as_deref(), Some("arena"));
        assert_eq!(parsed.netplay_player, Some(2));
        assert_eq!(parsed.netplay_input_delay_frames, Some(3));
        assert_eq!(parsed.netplay_max_rollback_frames, Some(360));
        assert_eq!(parsed.netplay_hash_check_every_frames, Some(90));
    }

    #[test]
    fn parse_runtime_args_control_flow_branches_do_not_hang() {
        let mcp_host = parse_runtime_args_with_timeout(vec!["--mcp-host".to_owned()])
            .expect("mcp host branch should parse");
        assert!(mcp_host.mcp_enabled);

        let netplay = parse_runtime_args_with_timeout(vec!["--netplay".to_owned()])
            .expect("netplay branch should parse");
        assert!(netplay.netplay_enabled);

        let relay = parse_runtime_args_with_timeout(vec![
            "--netplay-relay".to_owned(),
            "relay.example:4545".to_owned(),
        ])
        .expect("netplay relay branch should parse");
        assert_eq!(
            relay.netplay_relay_addr.as_deref(),
            Some("relay.example:4545")
        );

        let room =
            parse_runtime_args_with_timeout(vec!["--netplay-room".to_owned(), "arena".to_owned()])
                .expect("netplay room branch should parse");
        assert_eq!(room.netplay_room.as_deref(), Some("arena"));

        let player =
            parse_runtime_args_with_timeout(vec!["--netplay-player".to_owned(), "2".to_owned()])
                .expect("netplay player branch should parse");
        assert_eq!(player.netplay_player, Some(2));

        let delay =
            parse_runtime_args_with_timeout(vec!["--netplay-delay".to_owned(), "3".to_owned()])
                .expect("netplay delay branch should parse");
        assert_eq!(delay.netplay_input_delay_frames, Some(3));

        let hash_every = parse_runtime_args_with_timeout(vec![
            "--netplay-hash-every".to_owned(),
            "90".to_owned(),
        ])
        .expect("netplay hash-every branch should parse");
        assert_eq!(hash_every.netplay_hash_check_every_frames, Some(90));

        let bind_equals =
            parse_runtime_args_with_timeout(vec!["--mcp-bind=127.0.0.1:7777".to_owned()])
                .expect("mcp bind equals branch should parse");
        assert_eq!(bind_equals.mcp_bind_addr, "127.0.0.1:7777");

        let relay_equals =
            parse_runtime_args_with_timeout(vec!["--netplay-relay=relay.example:4545".to_owned()])
                .expect("netplay relay equals branch should parse");
        assert_eq!(
            relay_equals.netplay_relay_addr.as_deref(),
            Some("relay.example:4545")
        );

        let room_equals = parse_runtime_args_with_timeout(vec!["--netplay-room=arena".to_owned()])
            .expect("netplay room equals branch should parse");
        assert_eq!(room_equals.netplay_room.as_deref(), Some("arena"));

        let player_equals = parse_runtime_args_with_timeout(vec!["--netplay-player=2".to_owned()])
            .expect("netplay player equals branch should parse");
        assert_eq!(player_equals.netplay_player, Some(2));

        let delay_equals = parse_runtime_args_with_timeout(vec!["--netplay-delay=3".to_owned()])
            .expect("netplay delay equals branch should parse");
        assert_eq!(delay_equals.netplay_input_delay_frames, Some(3));
    }

    #[test]
    fn netplay_runtime_stats_tracks_rtt_jitter_rollbacks_and_desyncs() {
        let mut stats = NetplayRuntimeStats::new(2);
        assert_eq!(stats.latest_rtt_ms_or_zero(), 0.0);
        assert_eq!(stats.input_delay_frames, 2);
        assert_eq!(stats.rollback_count, 0);
        assert_eq!(stats.max_rollback_distance, 0);
        assert_eq!(stats.desync_count, 0);
        assert_eq!(stats.jitter_ms, 0.0);

        stats.observe_rtt_ms(20.0);
        assert_eq!(stats.latest_rtt_ms_or_zero(), 20.0);
        assert_eq!(stats.jitter_ms, 0.0);

        stats.observe_rtt_ms(28.0);
        assert_eq!(stats.latest_rtt_ms_or_zero(), 28.0);
        assert_eq!(stats.jitter_ms, 8.0);

        stats.observe_rtt_ms(40.0);
        assert!((stats.jitter_ms - 8.5).abs() < 1e-9);

        stats.observe_rollback(0);
        assert_eq!(stats.rollback_count, 0);
        stats.observe_rollback(2);
        stats.observe_rollback(5);
        assert_eq!(stats.rollback_count, 2);
        assert_eq!(stats.max_rollback_distance, 5);

        stats.observe_desync();
        stats.observe_desync();
        assert_eq!(stats.desync_count, 2);
    }

    #[test]
    fn perf_metrics_on_step_tracks_stalls_and_recovers_on_pc_change() {
        let core = NesCore::new();
        let mut metrics = PerfMetrics::new(true, 0, 0);
        assert_eq!(metrics.every_n_frames, DEFAULT_METRICS_EVERY_FRAMES);

        metrics.on_step(&core, Duration::from_millis(4), true);
        assert_eq!(metrics.report_frames, 1);
        assert_eq!(metrics.step_work, Duration::from_millis(4));
        assert_eq!(metrics.late_frames, 1);
        assert_eq!(metrics.last_pc, Some(core.cpu_pc()));
        assert_eq!(metrics.pc_stall_frames, 0);

        metrics.on_step(&core, Duration::from_millis(2), false);
        assert_eq!(metrics.report_frames, 2);
        assert_eq!(metrics.step_work, Duration::from_millis(6));
        assert_eq!(metrics.pc_stall_frames, 1);

        metrics.pc_stall_frames = 239;
        metrics.last_pc = Some(core.cpu_pc());
        metrics.warned_stall = false;
        metrics.on_step(&core, Duration::from_millis(1), false);
        assert_eq!(metrics.pc_stall_frames, 240);
        assert!(metrics.warned_stall);

        metrics.last_pc = Some(core.cpu_pc().wrapping_add(1));
        metrics.pc_stall_frames = 77;
        metrics.warned_stall = true;
        metrics.on_step(&core, Duration::from_millis(1), false);
        assert_eq!(metrics.pc_stall_frames, 0);
        assert!(!metrics.warned_stall);
    }

    #[test]
    fn perf_metrics_render_audio_and_netplay_observation_update_fields() {
        let core = NesCore::new();
        let mut metrics = PerfMetrics::new(true, 2, core.ppu_frame_counter());
        let frame = vec![0_u8; 256 * 240 * 4];

        metrics.on_render(&frame, Duration::from_millis(3));
        assert_eq!(metrics.render_work, Duration::from_millis(3));
        assert_eq!(metrics.unchanged_frame_count, 0);

        metrics.on_render(&frame, Duration::from_millis(2));
        assert_eq!(metrics.render_work, Duration::from_millis(5));
        assert_eq!(metrics.unchanged_frame_count, 1);

        metrics.on_audio_queue(3, false);
        metrics.on_audio_queue(2, true);
        assert_eq!(metrics.audio_queue_peak, 3);
        assert_eq!(metrics.audio_queue_drops, 1);

        let mut net = NetplayRuntimeStats::new(4);
        net.observe_rtt_ms(42.0);
        net.observe_rtt_ms(46.0);
        net.observe_rollback(3);
        net.observe_desync();
        metrics.on_netplay_stats(&net);
        assert_eq!(metrics.netplay_rtt_ms, 46.0);
        assert_eq!(metrics.netplay_jitter_ms, 4.0);
        assert_eq!(metrics.netplay_rollbacks, 1);
        assert_eq!(metrics.netplay_max_rollback_distance, 3);
        assert_eq!(metrics.netplay_desyncs, 1);
        assert_eq!(metrics.netplay_input_delay_frames, 4);
    }

    #[test]
    fn perf_metrics_maybe_report_resets_window_after_threshold() {
        let core = NesCore::new();
        let mut metrics = PerfMetrics::new(true, 2, core.ppu_frame_counter());
        metrics.report_frames = 2;
        metrics.step_work = Duration::from_millis(8);
        metrics.render_work = Duration::from_millis(6);
        metrics.late_frames = 1;
        metrics.audio_queue_peak = 4;
        metrics.audio_queue_drops = 2;
        metrics.netplay_rollbacks = 5;
        metrics.netplay_max_rollback_distance = 7;
        metrics.netplay_desyncs = 3;
        metrics.report_start = Instant::now() - Duration::from_millis(20);

        metrics.maybe_report(&core);

        assert_eq!(metrics.report_frames, 0);
        assert_eq!(metrics.step_work, Duration::ZERO);
        assert_eq!(metrics.render_work, Duration::ZERO);
        assert_eq!(metrics.late_frames, 0);
        assert_eq!(metrics.audio_queue_peak, 0);
        assert_eq!(metrics.audio_queue_drops, 0);
        assert_eq!(metrics.netplay_rollbacks, 0);
        assert_eq!(metrics.netplay_max_rollback_distance, 0);
        assert_eq!(metrics.netplay_desyncs, 0);
    }

    #[test]
    fn perf_metrics_maybe_report_guard_paths_skip_when_disabled_or_under_threshold() {
        let core = NesCore::new();

        let mut disabled = PerfMetrics::new(false, 1, core.ppu_frame_counter());
        disabled.report_frames = 1;
        disabled.step_work = Duration::from_millis(4);
        disabled.report_start = Instant::now() - Duration::from_millis(20);
        disabled.maybe_report(&core);
        assert_eq!(disabled.report_frames, 1);
        assert_eq!(disabled.step_work, Duration::from_millis(4));

        let mut under_threshold = PerfMetrics::new(true, 5, core.ppu_frame_counter());
        under_threshold.report_frames = 2;
        under_threshold.step_work = Duration::from_millis(6);
        under_threshold.report_start = Instant::now() - Duration::from_millis(20);
        under_threshold.maybe_report(&core);
        assert_eq!(under_threshold.report_frames, 2);
        assert_eq!(under_threshold.step_work, Duration::from_millis(6));
    }

    #[test]
    fn perf_metrics_disabled_mode_does_not_mutate_tracking_fields() {
        let core = NesCore::new();
        let frame = vec![0_u8; 256 * 240 * 4];
        let mut metrics = PerfMetrics::new(false, 1, 0);

        metrics.on_step(&core, Duration::from_millis(3), true);
        metrics.on_render(&frame, Duration::from_millis(2));
        metrics.on_audio_queue(10, true);
        let mut net = NetplayRuntimeStats::new(3);
        net.observe_rtt_ms(10.0);
        net.observe_rollback(2);
        net.observe_desync();
        metrics.on_netplay_stats(&net);
        metrics.maybe_report(&core);

        assert_eq!(metrics.report_frames, 0);
        assert_eq!(metrics.step_work, Duration::ZERO);
        assert_eq!(metrics.render_work, Duration::ZERO);
        assert_eq!(metrics.audio_queue_peak, 0);
        assert_eq!(metrics.audio_queue_drops, 0);
        assert_eq!(metrics.netplay_rtt_ms, 0.0);
        assert_eq!(metrics.netplay_rollbacks, 0);
        assert_eq!(metrics.netplay_desyncs, 0);
    }

    #[test]
    fn advance_core_for_host_frame_steps_cpu_budget() {
        let mut rom = sample_ines(0, 1);
        let prg_start = 16;
        rom[prg_start] = 0xA9; // LDA #$42
        rom[prg_start + 1] = 0x42;
        rom[prg_start + 0x3FFC] = 0x00; // reset vector low
        rom[prg_start + 0x3FFD] = 0x80; // reset vector high

        let mut core = NesCore::new();
        core.load_ines_rom(&rom).expect("sample rom should load");
        assert_eq!(core.cpu_a(), 0x00);
        assert_eq!(core.cpu_pc(), 0x8000);

        advance_core_for_host_frame(&mut core, StepMode::CpuBudget(1))
            .expect("cpu budget stepping should succeed");
        assert_eq!(core.cpu_a(), 0x42);
        assert_eq!(core.cpu_pc(), 0x8002);
    }

    #[test]
    fn frame_signature_matches_reference_and_changes_on_sampled_byte() {
        let mut frame = vec![0_u8; 256];
        let signature_a = frame_signature(&frame);

        let mut reference = 0xcbf2_9ce4_8422_2325_u64;
        for idx in (0..frame.len()).step_by(64) {
            reference ^= u64::from(frame[idx]);
            reference = reference.wrapping_mul(0x0000_0001_0000_01b3);
        }
        assert_eq!(signature_a, reference);

        frame[64] = 7;
        let signature_b = frame_signature(&frame);
        assert_ne!(signature_a, signature_b);
    }

    #[test]
    fn capture_config_helpers_handle_placeholders_and_defaults() {
        assert!(capture_config_from_parts(None, 10).is_none());
        assert!(capture_config_from_parts(Some("   ".to_owned()), 10).is_none());

        let cfg = capture_config_from_parts(Some("snap-{frame}.ppm".to_owned()), 0)
            .expect("valid template should produce config");
        assert_eq!(cfg.path_template, "snap-{frame}.ppm");
        assert_eq!(cfg.every_n_frames, DEFAULT_CAPTURE_EVERY_FRAMES);

        assert_eq!(
            capture_path_for_frame("snap-{frame}.ppm", 42),
            "snap-000042.ppm"
        );
        assert_eq!(capture_path_for_frame("snap.ppm", 42), "snap.ppm");
    }

    #[test]
    fn encode_ppm_emits_expected_headers_and_pixel_layout() {
        let ppm = encode_ppm(2, 1, &[1, 2, 3, 255, 4, 5, 6, 255]);
        assert!(ppm.starts_with(b"P6\n2 1\n255\n"));
        assert!(ppm.ends_with(&[1, 2, 3, 4, 5, 6]));
    }

    #[test]
    fn format_rom_read_error_handles_not_found_and_other_errors() {
        let not_found = std::io::Error::from(std::io::ErrorKind::NotFound);
        let msg = format_rom_read_error("bad.nes", &not_found);
        assert!(msg.contains("Could not find the ROM file at 'bad.nes'"));
        assert!(msg.contains("homebrew.nes"));

        let other = std::io::Error::from(std::io::ErrorKind::PermissionDenied);
        let msg = format_rom_read_error("bad.nes", &other);
        assert!(msg.contains("Failed to read ROM at 'bad.nes'"));
        assert!(msg.contains("permission denied"));
    }

    #[test]
    fn write_frame_ppm_validates_frame_size_and_writes_output_files() {
        let bad = write_frame_ppm("ignored.ppm", &[0_u8; 3]).expect_err("invalid size should fail");
        assert!(bad.contains("frame length mismatch"));

        let frame = vec![0_u8; 256 * 240 * 4];
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time should be monotonic")
            .as_nanos();
        let ppm_path = std::env::temp_dir().join(format!("nes-desktop-{nonce}.ppm"));
        let bmp_path = std::env::temp_dir().join(format!("nes-desktop-{nonce}.bmp"));

        write_frame_ppm(&ppm_path.to_string_lossy(), &frame).expect("ppm write should succeed");
        write_frame_ppm(&bmp_path.to_string_lossy(), &frame).expect("bmp write should succeed");

        let ppm_bytes = fs::read(&ppm_path).expect("ppm bytes should be readable");
        let bmp_bytes = fs::read(&bmp_path).expect("bmp bytes should be readable");
        assert!(ppm_bytes.starts_with(b"P6\n256 240\n255\n"));
        assert_eq!(&bmp_bytes[0..2], b"BM");

        let _ = fs::remove_file(ppm_path);
        let _ = fs::remove_file(bmp_path);
    }
}
