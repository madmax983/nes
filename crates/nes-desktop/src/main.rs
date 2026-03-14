use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
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
    RomLoadInfo,
};
use nes_desktop::actions::AppAction;
use nes_desktop::app::{map_key_event_to_button_bit, map_key_event_to_command};
use nes_desktop::manual_state::{
    SaveSlotMetadata, SaveSlotStatus, load_state_file, read_slot_metadata, save_state_file,
    slot_path_for_rom, slot_paths_for_rom,
};
use nes_desktop::menu::{
    DesktopMenu, build_native_menu, native_menu_supported, pick_rom_path, rom_picker_supported,
};
use nes_desktop::overlay::{
    OverlayCheatSummary, OverlayCommand, OverlayModel, OverlaySlotSummary, draw_overlay,
};
use nes_desktop::rta::{
    CalibrationRecorder, DEFAULT_RTA_PROFILES_DIR, DEFAULT_RTA_RUNS_DIR, ForbiddenAction,
    ProfileStatus, RtaEvent, RtaManager, RtaProfile, compute_rom_hash, load_profiles,
    select_profile,
};
use nes_desktop::session_cheats::SessionCheats;
use nes_netplay::{HashComparison, RollbackConfig, RollbackEngine, ServerMessage};
use nes_rewind::worker::{TimeMachine, TimeMachineConfig};
use pixels::{Pixels, SurfaceTexture};
use rodio::{OutputStream, Sink, buffer::SamplesBuffer};
use winit::dpi::LogicalSize;
use winit::event::{ElementState, Event, VirtualKeyCode, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoopBuilder};
use winit::window::{Window, WindowBuilder};

#[cfg(target_os = "macos")]
use winit::platform::macos::EventLoopBuilderExtMacOS;

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
const SAVE_SLOT_COUNT: u8 = 5;
const RUNTIME_USAGE: &str = "Usage: nes-desktop [--config <path>] [--cheat-code <code>] [--mcp-host] [--mcp-bind <addr>] [--netplay] [--netplay-relay <addr>] [--netplay-room <room>] [--netplay-player <1|2>] [--netplay-delay <frames>] [--netplay-max-rollback <frames>] [--netplay-hash-every <frames>] [--rta] [--rta-profile <id>] [--rta-profiles-dir <path>] [--rta-runs-dir <path>] [--rta-calibrate] [rom_path]";
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
    cheat_codes: Vec<String>,
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
    cheat_codes: Vec<String>,
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

#[derive(Debug, Clone)]
struct CaptureConfig {
    path_template: String,
    every_n_frames: u64,
}

struct LoadedRomSession {
    rom_path: PathBuf,
    rom_hash: String,
    info: RomLoadInfo,
    slot_metadata: Vec<SaveSlotMetadata>,
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
    ToggleOverlay,
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

fn classify_keyboard_input(
    key: VirtualKeyCode,
    pressed: bool,
    rollback_enabled: bool,
    rta_enabled: bool,
    rta_calibrate: bool,
) -> KeyboardDecision {
    if key == VirtualKeyCode::Escape && pressed {
        return KeyboardDecision::ToggleOverlay;
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
    if rta_enabled && pressed && key == VirtualKeyCode::F9 {
        return KeyboardDecision::RtaManualSplit;
    }
    if rta_enabled && rta_calibrate && pressed && key == VirtualKeyCode::F10 {
        return KeyboardDecision::RtaFinish;
    }

    let Some(key_code) = map_virtual_keycode(key) else {
        return KeyboardDecision::Noop;
    };

    if rollback_enabled {
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

fn slot_action_for_hotkey(is_save: bool, selected_slot: u8) -> Option<AppAction> {
    if !(1..=5).contains(&selected_slot) {
        return None;
    }
    Some(if is_save {
        AppAction::SaveSlot(selected_slot)
    } else {
        AppAction::LoadSlot(selected_slot)
    })
}

fn apply_runtime_cheat_codes(core: &mut NesCore, cheat_codes: &[String]) -> Result<(), String> {
    core.clear_cheat_codes();
    for raw_code in cheat_codes {
        core.add_cheat_code(raw_code)
            .map_err(|err| format!("Invalid cheat code '{raw_code}': {err}"))?;
    }
    Ok(())
}

fn apply_session_cheats(core: &mut NesCore, cheats: &SessionCheats) -> Result<(), String> {
    apply_runtime_cheat_codes(core, &cheats.enabled_codes())
}

fn load_rom_session(
    core: &mut NesCore,
    rom_path: &Path,
    cheats: &SessionCheats,
) -> Result<LoadedRomSession, String> {
    let rom_bytes = fs::read(rom_path)
        .map_err(|err| format_rom_read_error(&rom_path.display().to_string(), &err))?;
    core.clear_cheat_codes();
    let info = core
        .load_ines_rom(&rom_bytes)
        .map_err(|err| format!("Failed to load ROM: {err}"))?;
    apply_session_cheats(core, cheats)?;
    let rom_hash = compute_rom_hash(&rom_bytes);
    let slot_metadata = load_slot_metadata_for_rom(rom_path, &rom_hash)?;
    Ok(LoadedRomSession {
        rom_path: rom_path.to_path_buf(),
        rom_hash,
        info,
        slot_metadata,
    })
}

fn load_slot_metadata_for_rom(
    rom_path: &Path,
    rom_hash: &str,
) -> Result<Vec<SaveSlotMetadata>, String> {
    slot_paths_for_rom(rom_path, rom_hash, 1..=SAVE_SLOT_COUNT)
        .into_iter()
        .map(|path| read_slot_metadata(&path, rom_hash))
        .collect()
}

fn refresh_slot_metadata(session: &mut LoadedRomSession) -> Result<(), String> {
    session.slot_metadata = load_slot_metadata_for_rom(&session.rom_path, &session.rom_hash)?;
    Ok(())
}

fn slot_path_for_selection(session: &LoadedRomSession, slot: u8) -> PathBuf {
    slot_path_for_rom(&session.rom_path, &session.rom_hash, slot)
}

fn format_slot_status(metadata: &SaveSlotMetadata) -> OverlaySlotSummary {
    let status_label = match metadata.status {
        SaveSlotStatus::Empty => "Empty",
        SaveSlotStatus::Saved => "Saved",
        SaveSlotStatus::Corrupt => "Corrupt",
        SaveSlotStatus::IncompatibleRom => "Mismatch",
    }
    .to_owned();
    let detail = metadata.modified_unix_secs.map(|secs| secs.to_string());
    OverlaySlotSummary {
        slot: metadata.slot,
        status_label,
        detail,
    }
}

fn overlay_slot_summaries(metadata: &[SaveSlotMetadata]) -> Vec<OverlaySlotSummary> {
    metadata.iter().map(format_slot_status).collect()
}

fn overlay_cheat_summaries(cheats: &SessionCheats) -> Vec<OverlayCheatSummary> {
    cheats
        .entries()
        .iter()
        .map(|entry| OverlayCheatSummary {
            raw_code: entry.raw_code.clone(),
            enabled: entry.enabled,
        })
        .collect()
}

fn rom_display_name(path: &Path) -> String {
    path.file_name()
        .and_then(|name| name.to_str())
        .filter(|name| !name.is_empty())
        .unwrap_or("ROM")
        .to_owned()
}

fn window_title(session: &LoadedRomSession, overlay_open: bool) -> String {
    let suffix = if overlay_open { " [Paused]" } else { "" };
    format!(
        "nes-desktop - {}{suffix}",
        rom_display_name(&session.rom_path)
    )
}

fn apply_overlay_keyboard_input(
    overlay: &mut OverlayModel,
    key: VirtualKeyCode,
    pressed: bool,
    cheat_count: usize,
    _keyboard_bits: &mut u8,
) -> Option<OverlayCommand> {
    overlay.handle_key(key, pressed, cheat_count)
}

fn validate_action_allowed(action: AppAction, rollback_enabled: bool) -> Result<(), String> {
    if rollback_enabled
        && matches!(
            action,
            AppAction::OpenRom
                | AppAction::OpenCheats
                | AppAction::SaveSlot(_)
                | AppAction::LoadSlot(_)
        )
    {
        return Err(
            "manual menu action is unavailable while netplay/rollback is active".to_owned(),
        );
    }
    Ok(())
}

fn overlay_input_requires_redraw(key: VirtualKeyCode, pressed: bool) -> bool {
    pressed
        && (matches!(
            key,
            VirtualKeyCode::Up
                | VirtualKeyCode::Down
                | VirtualKeyCode::Escape
                | VirtualKeyCode::Return
                | VirtualKeyCode::Space
                | VirtualKeyCode::Delete
                | VirtualKeyCode::Back
                | VirtualKeyCode::F5
                | VirtualKeyCode::F8
        ) || matches!(
            key,
            VirtualKeyCode::A
                | VirtualKeyCode::E
                | VirtualKeyCode::G
                | VirtualKeyCode::I
                | VirtualKeyCode::K
                | VirtualKeyCode::L
                | VirtualKeyCode::N
                | VirtualKeyCode::O
                | VirtualKeyCode::P
                | VirtualKeyCode::S
                | VirtualKeyCode::T
                | VirtualKeyCode::U
                | VirtualKeyCode::V
                | VirtualKeyCode::X
                | VirtualKeyCode::Y
                | VirtualKeyCode::Z
        ))
}

fn menu_action_enabled(
    action: AppAction,
    overlay_open: bool,
    rollback_enabled: bool,
    rta_active: bool,
) -> bool {
    match action {
        AppAction::Resume => overlay_open,
        AppAction::OpenRom => !rollback_enabled && !rta_active && rom_picker_supported(),
        AppAction::OpenCheats => !rollback_enabled && !rta_active,
        AppAction::SaveSlot(_) | AppAction::LoadSlot(_) => !rollback_enabled,
        AppAction::ToggleOverlay | AppAction::Reset | AppAction::Quit => true,
    }
}

fn sync_native_menu_state(
    menu: &DesktopMenu,
    overlay_open: bool,
    rollback_enabled: bool,
    rta_active: bool,
) {
    menu.set_action_enabled(
        AppAction::Resume,
        menu_action_enabled(
            AppAction::Resume,
            overlay_open,
            rollback_enabled,
            rta_active,
        ),
    );
    menu.set_action_enabled(
        AppAction::OpenRom,
        menu_action_enabled(
            AppAction::OpenRom,
            overlay_open,
            rollback_enabled,
            rta_active,
        ),
    );
    menu.set_action_enabled(
        AppAction::OpenCheats,
        menu_action_enabled(
            AppAction::OpenCheats,
            overlay_open,
            rollback_enabled,
            rta_active,
        ),
    );
    for slot in 1..=SAVE_SLOT_COUNT {
        menu.set_action_enabled(
            AppAction::SaveSlot(slot),
            menu_action_enabled(
                AppAction::SaveSlot(slot),
                overlay_open,
                rollback_enabled,
                rta_active,
            ),
        );
        menu.set_action_enabled(
            AppAction::LoadSlot(slot),
            menu_action_enabled(
                AppAction::LoadSlot(slot),
                overlay_open,
                rollback_enabled,
                rta_active,
            ),
        );
    }
    menu.set_action_enabled(
        AppAction::Reset,
        menu_action_enabled(AppAction::Reset, overlay_open, rollback_enabled, rta_active),
    );
    menu.set_action_enabled(
        AppAction::Quit,
        menu_action_enabled(AppAction::Quit, overlay_open, rollback_enabled, rta_active),
    );
}

fn set_overlay_open(
    overlay: &mut OverlayModel,
    open: bool,
    core: &mut NesCore,
    audio_output: Option<&AudioOutput>,
    window: &Window,
    session: &LoadedRomSession,
) -> Result<(), String> {
    if open {
        overlay.open();
        reconcile_core_pause_with_overlay(core, true)?;
        if let Some(output) = audio_output {
            output.clear();
        }
    } else {
        overlay.close();
        reconcile_core_pause_with_overlay(core, false)?;
    }
    window.set_title(&window_title(session, overlay.is_open()));
    Ok(())
}

fn reconcile_core_pause_with_overlay(core: &mut NesCore, overlay_open: bool) -> Result<(), String> {
    let command = if overlay_open {
        Command::Pause
    } else {
        Command::Resume
    };
    core.execute(command).map_err(|err| {
        format!(
            "Failed to {} emulation: {err}",
            if overlay_open { "pause" } else { "resume" }
        )
    })
}

#[allow(clippy::too_many_arguments)]
fn dispatch_app_action(
    action: AppAction,
    core: &mut NesCore,
    session: &mut LoadedRomSession,
    session_cheats: &mut SessionCheats,
    overlay: &mut OverlayModel,
    rollback_enabled: bool,
    runtime: &RuntimeConfig,
    audio_output: Option<&AudioOutput>,
    time_machine: &mut TimeMachine,
    rewind_held: &mut bool,
    metrics: &mut PerfMetrics,
    keyboard_bits: u8,
    gamepad_bits: &mut [u8; 2],
    window: &Window,
    rta_manager: &mut Option<RtaManager>,
    frame_index: u64,
    control_flow: &mut ControlFlow,
) -> bool {
    match execute_app_action(
        action,
        AppActionContext {
            core,
            session,
            session_cheats,
            overlay,
            rollback_enabled,
            runtime,
            audio_output,
            time_machine,
            rewind_held,
            metrics,
            keyboard_bits,
            gamepad_bits,
            window,
            rta_manager,
            frame_index,
        },
    ) {
        Ok(true) => {
            *control_flow = ControlFlow::Exit;
            true
        }
        Ok(false) => {
            window.request_redraw();
            false
        }
        Err(err) => {
            overlay.set_status_message(err);
            let _ = set_overlay_open(overlay, true, core, audio_output, window, session);
            window.request_redraw();
            false
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn dispatch_overlay_command(
    command: OverlayCommand,
    core: &mut NesCore,
    session: &mut LoadedRomSession,
    session_cheats: &mut SessionCheats,
    overlay: &mut OverlayModel,
    rollback_enabled: bool,
    runtime: &RuntimeConfig,
    audio_output: Option<&AudioOutput>,
    time_machine: &mut TimeMachine,
    rewind_held: &mut bool,
    metrics: &mut PerfMetrics,
    keyboard_bits: u8,
    gamepad_bits: &mut [u8; 2],
    window: &Window,
    rta_manager: &mut Option<RtaManager>,
    frame_index: u64,
    control_flow: &mut ControlFlow,
) -> bool {
    match command {
        OverlayCommand::AppAction(action) => dispatch_app_action(
            action,
            core,
            session,
            session_cheats,
            overlay,
            rollback_enabled,
            runtime,
            audio_output,
            time_machine,
            rewind_held,
            metrics,
            keyboard_bits,
            gamepad_bits,
            window,
            rta_manager,
            frame_index,
            control_flow,
        ),
        OverlayCommand::ToggleCheat(index) => {
            let Some(raw_code) = session_cheats
                .entries()
                .get(index)
                .map(|entry| entry.raw_code.clone())
            else {
                overlay.set_status_message(format!("No cheat entry exists at index {index}"));
                window.request_redraw();
                return false;
            };
            match session_cheats.toggle(index) {
                Ok(()) => {
                    if let Err(err) = apply_session_cheats(core, session_cheats) {
                        overlay.set_status_message(err);
                    } else {
                        let enabled = session_cheats
                            .entries()
                            .get(index)
                            .is_some_and(|entry| entry.enabled);
                        overlay.set_status_message(format!(
                            "[cheat] {} {raw_code}",
                            if enabled { "enabled" } else { "disabled" }
                        ));
                    }
                }
                Err(err) => overlay.set_status_message(err.to_string()),
            }
            window.request_redraw();
            false
        }
        OverlayCommand::RemoveCheat(index) => {
            match session_cheats.remove(index) {
                Ok(removed) => {
                    if let Err(err) = apply_session_cheats(core, session_cheats) {
                        overlay.set_status_message(err);
                    } else {
                        overlay.set_status_message(format!("[cheat] removed {}", removed.raw_code));
                    }
                }
                Err(err) => overlay.set_status_message(err.to_string()),
            }
            window.request_redraw();
            false
        }
        OverlayCommand::SubmitCheatCode(raw_code) => {
            match session_cheats.add(&raw_code) {
                Ok(()) => {
                    if let Err(err) = apply_session_cheats(core, session_cheats) {
                        overlay.set_status_message(err);
                    } else {
                        let new_index = session_cheats.len().saturating_sub(1);
                        overlay.close_add_cheat_modal();
                        overlay.focus_cheat(new_index);
                        overlay.set_status_message(format!(
                            "[cheat] added {}",
                            session_cheats.entries()[new_index].raw_code
                        ));
                    }
                }
                Err(err) => overlay
                    .set_status_message(format!("Invalid cheat code '{}': {err}", raw_code.trim())),
            }
            window.request_redraw();
            false
        }
    }
}

struct AppActionContext<'a> {
    core: &'a mut NesCore,
    session: &'a mut LoadedRomSession,
    session_cheats: &'a mut SessionCheats,
    overlay: &'a mut OverlayModel,
    rollback_enabled: bool,
    runtime: &'a RuntimeConfig,
    audio_output: Option<&'a AudioOutput>,
    time_machine: &'a mut TimeMachine,
    rewind_held: &'a mut bool,
    metrics: &'a mut PerfMetrics,
    keyboard_bits: u8,
    gamepad_bits: &'a mut [u8; 2],
    window: &'a Window,
    rta_manager: &'a mut Option<RtaManager>,
    frame_index: u64,
}

fn execute_app_action(
    action: AppAction,
    ctx: AppActionContext<'_>,
) -> Result<bool, String> {
    validate_action_allowed(action, ctx.rollback_enabled)?;

    match action {
        AppAction::ToggleOverlay => {
            set_overlay_open(
                ctx.overlay,
                !ctx.overlay.is_open(),
                ctx.core,
                ctx.audio_output,
                ctx.window,
                ctx.session,
            )?;
            Ok(false)
        }
        AppAction::Resume => {
            set_overlay_open(ctx.overlay, false, ctx.core, ctx.audio_output, ctx.window, ctx.session)?;
            Ok(false)
        }
        AppAction::OpenCheats => {
            if ctx.rta_manager.is_some() {
                ctx.overlay.set_status_message("Cheats are unavailable while RTA mode is active");
                return Ok(false);
            }
            if !ctx.overlay.is_open() {
                set_overlay_open(ctx.overlay, true, ctx.core, ctx.audio_output, ctx.window, ctx.session)?;
            }
            ctx.overlay.open_cheats_panel();
            ctx.window.set_title(&window_title(ctx.session, true));
            Ok(false)
        }
        AppAction::OpenRom => {
            if ctx.rta_manager.is_some() {
                ctx.overlay.set_status_message("Open ROM is unavailable while RTA mode is active");
                return Ok(false);
            }
            if !rom_picker_supported() {
                ctx.overlay.set_status_message("Open ROM picker is unavailable on this platform build");
                return Ok(false);
            }
            let Some(path) = pick_rom_path() else {
                ctx.overlay.set_status_message("Open ROM cancelled");
                return Ok(false);
            };
            let cleared_cheats = SessionCheats::new();
            *ctx.session = load_rom_session(ctx.core, &path, &cleared_cheats)?;
            ctx.session_cheats.clear();
            if let Some(output) = ctx.audio_output {
                output.clear();
            }
            *ctx.rewind_held = false;
            *ctx.time_machine = TimeMachine::new(TimeMachineConfig::default());
            ctx.time_machine.record_frame(ctx.core);
            *ctx.metrics = PerfMetrics::new(
                ctx.runtime.metrics_enabled,
                ctx.runtime.metrics_every_frames,
                ctx.core.ppu_frame_counter(),
            );
            resync_restored_inputs(ctx.core, ctx.keyboard_bits, ctx.gamepad_bits)?;
            ctx.overlay.clear_status_message();
            set_overlay_open(ctx.overlay, false, ctx.core, ctx.audio_output, ctx.window, ctx.session)?;
            Ok(false)
        }
        AppAction::SaveSlot(slot) => {
            if let Some(rta) = ctx.rta_manager.as_mut() {
                let _ = rta.mark_forbidden_action(
                    ForbiddenAction::SaveLoad,
                    ctx.frame_index,
                    Instant::now(),
                );
            }
            let snapshot = ctx.core.save_state();
            let slot_path = slot_path_for_selection(ctx.session, slot);
            save_state_file(&slot_path, &ctx.session.rom_hash, &snapshot)?;
            refresh_slot_metadata(ctx.session)?;
            ctx.overlay.focus_slot(slot, true);
            ctx.overlay.set_status_message(format!("[state] saved {}", slot_path.display()));
            Ok(false)
        }
        AppAction::LoadSlot(slot) => {
            if let Some(rta) = ctx.rta_manager.as_mut() {
                let _ = rta.mark_forbidden_action(
                    ForbiddenAction::SaveLoad,
                    ctx.frame_index,
                    Instant::now(),
                );
            }
            let slot_path = slot_path_for_selection(ctx.session, slot);
            let snapshot = load_state_file(&slot_path, &ctx.session.rom_hash)?;
            ctx.core.load_state(&snapshot);
            apply_session_cheats(ctx.core, ctx.session_cheats)?;
            reconcile_core_pause_with_overlay(ctx.core, ctx.overlay.is_open())?;
            resync_restored_inputs(ctx.core, ctx.keyboard_bits, ctx.gamepad_bits)?;
            if let Some(output) = ctx.audio_output {
                output.clear();
            }
            *ctx.rewind_held = false;
            *ctx.time_machine = TimeMachine::new(TimeMachineConfig::default());
            ctx.time_machine.record_frame(ctx.core);
            *ctx.metrics = PerfMetrics::new(
                ctx.runtime.metrics_enabled,
                ctx.runtime.metrics_every_frames,
                ctx.core.ppu_frame_counter(),
            );
            refresh_slot_metadata(ctx.session)?;
            ctx.overlay.focus_slot(slot, false);
            ctx.overlay.set_status_message(format!("[state] loaded {}", slot_path.display()));
            Ok(false)
        }
        AppAction::Reset => {
            ctx.core.execute(Command::Reset)
                .map_err(|err| format!("Reset failed: {err}"))?;
            *ctx.rewind_held = false;
            *ctx.time_machine = TimeMachine::new(TimeMachineConfig::default());
            ctx.time_machine.record_frame(ctx.core);
            *ctx.metrics = PerfMetrics::new(
                ctx.runtime.metrics_enabled,
                ctx.runtime.metrics_every_frames,
                ctx.core.ppu_frame_counter(),
            );
            ctx.overlay.set_status_message("System reset");
            set_overlay_open(ctx.overlay, false, ctx.core, ctx.audio_output, ctx.window, ctx.session)?;
            Ok(false)
        }
        AppAction::Quit => Ok(true),
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

    let mut core = NesCore::new();
    let mut session_cheats = SessionCheats::from_raw_codes(&runtime.cheat_codes)
        .map_err(|err| format!("Invalid cheat code in runtime config: {err}"))?;
    let mut session = load_rom_session(&mut core, Path::new(&runtime.rom_path), &session_cheats)?;
    let step_mode = runtime.step_mode;
    let mut rta_manager = if let Some(rta_config) = runtime.rta.as_ref() {
        let profiles = load_profiles(&rta_config.profiles_dir)?;
        let profile = if rta_config.calibrate {
            match select_profile(
                &profiles,
                &session.rom_hash,
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
                            rom_hashes: vec![session.rom_hash.clone()],
                            status: ProfileStatus::Published,
                            ..RtaProfile::default()
                        }
                    } else {
                        return Err(format!(
                            "RTA calibration requires --rta-profile <id> when no existing profile matches ROM hash {}: {err}",
                            session.rom_hash
                        ));
                    }
                }
            }
        } else {
            select_profile(
                &profiles,
                &session.rom_hash,
                rta_config.profile_id_override.as_deref(),
                false,
            )
            .map_err(|err| {
                format!(
                    "Failed to enter RTA mode for ROM hash {}: {err}. Provide --rta-profile <id> to override.",
                    session.rom_hash
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
            session.rom_hash.clone(),
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
        Cell::new(session.rom_path.display().to_string()).fg(TableColor::Green),
    ]);
    table.add_row(vec![
        Cell::new("ROM Info"),
        Cell::new(format!(
            "Mapper {}, PRG {} bytes, reset vector ${:04X}",
            session.info.mapper_id, session.info.prg_rom_bytes, session.info.reset_pc
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
            "keyboard Z=A, X=B, Enter=Start, RightShift=Select, Arrows=D-pad, R=Rewind, F5=Save Slot, F8=Load Slot, Esc=Menu",
        ),
    ]);
    table.add_row(vec![
        Cell::new("Menu"),
        Cell::new(if native_menu_supported() {
            "native menu bar + Esc overlay"
        } else {
            "Esc overlay only on this platform"
        }),
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

    let mut event_loop_builder = EventLoopBuilder::new();
    #[cfg(target_os = "macos")]
    event_loop_builder.with_default_menu(false);
    let event_loop = event_loop_builder.build();
    let (window_width, window_height) = scaled_window_dimensions(runtime.window_scale);
    let window = WindowBuilder::new()
        .with_title(window_title(&session, false))
        .with_inner_size(LogicalSize::new(window_width, window_height))
        .with_min_inner_size(LogicalSize::new(FRAME_WIDTH as f64, FRAME_HEIGHT as f64))
        .build(&event_loop)
        .map_err(|err| format!("Failed to create window: {err}"))?;
    let desktop_menu = build_native_menu(SAVE_SLOT_COUNT);
    desktop_menu.install_for_window(&window)?;

    let window_size = window.inner_size();
    let surface_texture = SurfaceTexture::new(window_size.width, window_size.height, &window);
    let mut pixels = Pixels::new(FRAME_WIDTH as u32, FRAME_HEIGHT as u32, surface_texture)
        .map_err(|err| format!("Failed to create pixel surface: {err}"))?;

    let mut frame_index = 0_u64;
    let mut frame_rgba = vec![0_u8; FRAME_RGBA_BYTES];
    let mut next_frame_deadline = Instant::now();
    let capture = runtime.capture.clone();
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
    let mut overlay = OverlayModel::new(SAVE_SLOT_COUNT);
    sync_native_menu_state(
        &desktop_menu,
        overlay.is_open(),
        rollback.is_some(),
        rta_manager.is_some(),
    );

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
                if overlay.is_open() {
                    let action = apply_overlay_keyboard_input(
                        &mut overlay,
                        key,
                        pressed,
                        session_cheats.len(),
                        &mut keyboard_bits,
                    );
                    if overlay_input_requires_redraw(key, pressed) {
                        window.request_redraw();
                    }
                    if let Some(command) = action {
                        let _ = dispatch_overlay_command(
                            command,
                            &mut core,
                            &mut session,
                            &mut session_cheats,
                            &mut overlay,
                            rollback.is_some(),
                            &runtime,
                            audio_output.as_ref(),
                            &mut time_machine,
                            &mut rewind_held,
                            &mut metrics,
                            keyboard_bits,
                            &mut gamepad_bits,
                            &window,
                            &mut rta_manager,
                            frame_index,
                            control_flow,
                        );
                    }
                    return;
                }
                track_keyboard_bits_for_key(key, pressed, &mut keyboard_bits);
                match classify_keyboard_input(
                    key,
                    pressed,
                    rollback.is_some(),
                    rta_manager.is_some(),
                    rta_manager.as_ref().is_some_and(|manager| manager.is_calibrating()),
                ) {
                    KeyboardDecision::ToggleOverlay => {
                        let _ = dispatch_app_action(
                            AppAction::ToggleOverlay,
                            &mut core,
                            &mut session,
                            &mut session_cheats,
                            &mut overlay,
                            rollback.is_some(),
                            &runtime,
                            audio_output.as_ref(),
                            &mut time_machine,
                            &mut rewind_held,
                            &mut metrics,
                            keyboard_bits,
                            &mut gamepad_bits,
                            &window,
                            &mut rta_manager,
                            frame_index,
                            control_flow,
                        );
                    }
                    KeyboardDecision::ManualSaveState => {
                        if let Some(action) = slot_action_for_hotkey(true, overlay.selected_slot()) {
                            let _ = dispatch_app_action(
                                action,
                                &mut core,
                                &mut session,
                                &mut session_cheats,
                                &mut overlay,
                                rollback.is_some(),
                                &runtime,
                                audio_output.as_ref(),
                                &mut time_machine,
                                &mut rewind_held,
                                &mut metrics,
                                keyboard_bits,
                                &mut gamepad_bits,
                                &window,
                                &mut rta_manager,
                                frame_index,
                                control_flow,
                            );
                        }
                    }
                    KeyboardDecision::ManualLoadState => {
                        if let Some(action) = slot_action_for_hotkey(false, overlay.selected_slot()) {
                            let _ = dispatch_app_action(
                                action,
                                &mut core,
                                &mut session,
                                &mut session_cheats,
                                &mut overlay,
                                rollback.is_some(),
                                &runtime,
                                audio_output.as_ref(),
                                &mut time_machine,
                                &mut rewind_held,
                                &mut metrics,
                                keyboard_bits,
                                &mut gamepad_bits,
                                &window,
                                &mut rta_manager,
                                frame_index,
                                control_flow,
                            );
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

            sync_native_menu_state(
                &desktop_menu,
                overlay.is_open(),
                rollback.is_some(),
                rta_manager.is_some(),
            );
            while let Some(action) = desktop_menu.poll_action() {
                if dispatch_app_action(
                    action,
                    &mut core,
                    &mut session,
                    &mut session_cheats,
                    &mut overlay,
                    rollback.is_some(),
                    &runtime,
                    audio_output.as_ref(),
                    &mut time_machine,
                    &mut rewind_held,
                    &mut metrics,
                    keyboard_bits,
                    &mut gamepad_bits,
                    &window,
                    &mut rta_manager,
                    frame_index,
                    control_flow,
                ) {
                    return;
                }
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
                        && !overlay.is_open()
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

            if overlay.is_open() {
                *control_flow = ControlFlow::Wait;
                return;
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
            if overlay.is_open() {
                let slot_summaries = overlay_slot_summaries(&session.slot_metadata);
                let cheat_summaries = overlay_cheat_summaries(&session_cheats);
                draw_overlay(
                    pixels.frame_mut(),
                    FRAME_WIDTH,
                    FRAME_HEIGHT,
                    &overlay,
                    &slot_summaries,
                    &cheat_summaries,
                );
            }
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
        cheat_codes: runtime_args.cheat_codes,
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
        cheat_codes: Vec::new(),
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
                "{RUNTIME_USAGE}\nDefault config path: {DEFAULT_CONFIG_PATH}"
            ));
        }
        if arg == "--cheat-code" {
            let Some(code) = args.get(idx + 1) else {
                return Err("missing value after --cheat-code".to_owned());
            };
            parsed.cheat_codes.push(code.clone());
            idx += 2;
            continue;
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
        if let Some(code) = arg.strip_prefix("--cheat-code=") {
            if code.is_empty() {
                return Err("missing value after --cheat-code=".to_owned());
            }
            parsed.cheat_codes.push(code.to_owned());
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
            return Err(format!("unknown flag '{arg}'. {RUNTIME_USAGE}"));
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

