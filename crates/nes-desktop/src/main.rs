use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
use std::time::{Duration, Instant};

#[cfg(feature = "nova")]
mod auto_player;
pub(crate) mod config;
pub(crate) mod gamepad;
pub(crate) mod input;
pub(crate) mod metrics;
mod netplay;
pub(crate) mod session;
use crate::config::*;
use crate::metrics::PerfMetrics;
use crate::session::*;

use crate::gamepad::*;
use crate::input::*;
use comfy_table::{Cell, Color as TableColor, Table, presets::UTF8_FULL};
use crossterm::style::{Color, Stylize};
use gilrs::{Axis as GamepadAxis, Button as GamepadButton, GamepadId, Gilrs};
use nes_core::{Command, FRAME_HEIGHT, FRAME_RGBA_BYTES, FRAME_WIDTH, NesCore};
use nes_desktop::actions::AppAction;
use nes_desktop::app::map_key_event_to_button_bit;
use nes_desktop::audio::{AudioOutput, MAX_AUDIO_QUEUE_CHUNKS};
use nes_desktop::manual_state::{load_state_file, save_state_file};
use nes_desktop::menu::{
    DesktopMenu, build_native_menu, native_menu_supported, pick_rom_path, rom_picker_supported,
};
use nes_desktop::overlay::{OverlayCheatSummary, OverlayCommand, OverlayModel, draw_overlay};
use nes_desktop::rta::{
    CalibrationRecorder, ForbiddenAction, ProfileStatus, RtaEvent, RtaManager, RtaProfile,
    load_profiles, select_profile,
};
use nes_desktop::session_cheats::SessionCheats;
use nes_netplay::{RollbackConfig, RollbackEngine};
use nes_rewind::worker::{TimeMachine, TimeMachineConfig};
use pixels::{Pixels, SurfaceTexture};
use winit::dpi::LogicalSize;
use winit::event::{Event, VirtualKeyCode};
use winit::event_loop::{ControlFlow, EventLoopBuilder};
use winit::window::{Window, WindowBuilder};

#[cfg(feature = "mcp-host")]
use nes_desktop::mcp_host::McpHost;
#[cfg(target_os = "macos")]
use winit::platform::macos::EventLoopBuilderExtMacOS;

use crate::netplay::{NetplayClient, NetplayRuntimeStats};

const TARGET_FRAME_TIME: Duration = Duration::from_micros(16_667);
const NETPLAY_PING_INTERVAL: Duration = Duration::from_millis(500);
const NETPLAY_AUTO_DELAY_MIN_FRAMES: u32 = 1;
const NETPLAY_AUTO_DELAY_MAX_FRAMES: u32 = 12;

fn main() {
    if let Err(err) = run() {
        eprintln!("\n{}", err);
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
    let set_enabled = |action: AppAction| {
        menu.set_action_enabled(
            action,
            menu_action_enabled(action, overlay_open, rollback_enabled, rta_active),
        );
    };

    set_enabled(AppAction::Resume);
    set_enabled(AppAction::OpenRom);
    set_enabled(AppAction::OpenCheats);

    for slot in 1..=SAVE_SLOT_COUNT {
        set_enabled(AppAction::SaveSlot(slot));
        set_enabled(AppAction::LoadSlot(slot));
    }

    set_enabled(AppAction::Reset);
    set_enabled(AppAction::Quit);
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

struct AppContext<'a> {
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

fn dispatch_app_action(
    action: AppAction,
    ctx: &mut AppContext<'_>,
    control_flow: &mut ControlFlow,
) -> bool {
    match execute_app_action(action, ctx) {
        Ok(true) => {
            *control_flow = ControlFlow::Exit;
            true
        }
        Ok(false) => {
            ctx.window.request_redraw();
            false
        }
        Err(err) => {
            ctx.overlay.set_status_message(err);
            let _ = set_overlay_open(
                ctx.overlay,
                true,
                ctx.core,
                ctx.audio_output,
                ctx.window,
                ctx.session,
            );
            ctx.window.request_redraw();
            false
        }
    }
}

fn dispatch_overlay_command(
    command: OverlayCommand,
    ctx: &mut AppContext<'_>,
    control_flow: &mut ControlFlow,
) -> bool {
    match command {
        OverlayCommand::AppAction(action) => dispatch_app_action(action, ctx, control_flow),
        OverlayCommand::ToggleCheat(index) => {
            let Some(raw_code) = ctx
                .session_cheats
                .entries()
                .get(index)
                .map(|entry| entry.raw_code.clone())
            else {
                ctx.overlay
                    .set_status_message(format!("No cheat entry exists at index {index}"));
                ctx.window.request_redraw();
                return false;
            };
            match ctx.session_cheats.toggle(index) {
                Ok(()) => {
                    if let Err(err) = apply_session_cheats(ctx.core, ctx.session_cheats) {
                        ctx.overlay.set_status_message(err);
                    } else {
                        let enabled = ctx
                            .session_cheats
                            .entries()
                            .get(index)
                            .is_some_and(|entry| entry.enabled);
                        ctx.overlay.set_status_message(format!(
                            "[cheat] {} {raw_code}",
                            if enabled { "enabled" } else { "disabled" }
                        ));
                    }
                }
                Err(err) => ctx.overlay.set_status_message(err.to_string()),
            }
            ctx.window.request_redraw();
            false
        }
        OverlayCommand::RemoveCheat(index) => {
            match ctx.session_cheats.remove(index) {
                Ok(removed) => {
                    if let Err(err) = apply_session_cheats(ctx.core, ctx.session_cheats) {
                        ctx.overlay.set_status_message(err);
                    } else {
                        ctx.overlay
                            .set_status_message(format!("[cheat] removed {}", removed.raw_code));
                    }
                }
                Err(err) => ctx.overlay.set_status_message(err.to_string()),
            }
            ctx.window.request_redraw();
            false
        }
        OverlayCommand::SubmitCheatCode(raw_code) => {
            match ctx.session_cheats.add(&raw_code) {
                Ok(()) => {
                    if let Err(err) = apply_session_cheats(ctx.core, ctx.session_cheats) {
                        ctx.overlay.set_status_message(err);
                    } else {
                        let new_index = ctx.session_cheats.len().saturating_sub(1);
                        ctx.overlay.close_add_cheat_modal();
                        ctx.overlay.focus_cheat(new_index);
                        ctx.overlay.set_status_message(format!(
                            "[cheat] added {}",
                            ctx.session_cheats.entries()[new_index].raw_code
                        ));
                    }
                }
                Err(err) => ctx
                    .overlay
                    .set_status_message(format!("Invalid cheat code '{}': {err}", raw_code.trim())),
            }
            ctx.window.request_redraw();
            false
        }
    }
}

fn execute_app_action(action: AppAction, ctx: &mut AppContext<'_>) -> Result<bool, String> {
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
            set_overlay_open(
                ctx.overlay,
                false,
                ctx.core,
                ctx.audio_output,
                ctx.window,
                ctx.session,
            )?;
            Ok(false)
        }
        AppAction::OpenCheats => {
            if ctx.rta_manager.is_some() {
                ctx.overlay
                    .set_status_message("Cheats are unavailable while RTA mode is active");
                return Ok(false);
            }
            if !ctx.overlay.is_open() {
                set_overlay_open(
                    ctx.overlay,
                    true,
                    ctx.core,
                    ctx.audio_output,
                    ctx.window,
                    ctx.session,
                )?;
            }
            ctx.overlay.open_cheats_panel();
            ctx.window.set_title(&window_title(ctx.session, true));
            Ok(false)
        }
        AppAction::OpenRom => {
            if ctx.rta_manager.is_some() {
                ctx.overlay
                    .set_status_message("Open ROM is unavailable while RTA mode is active");
                return Ok(false);
            }
            if !rom_picker_supported() {
                ctx.overlay
                    .set_status_message("Open ROM picker is unavailable on this platform build");
                return Ok(false);
            }
            let Some(path) = pick_rom_path() else {
                ctx.overlay.set_status_message("Open ROM cancelled");
                return Ok(false);
            };
            let cleared_cheats = SessionCheats::new();
            *ctx.session = load_rom_session(ctx.core, &path, &cleared_cheats)?;
            ctx.session_cheats.clear();
            reset_ephemeral_state(ctx);
            resync_restored_inputs(ctx.core, ctx.keyboard_bits, ctx.gamepad_bits)?;
            ctx.overlay.clear_status_message();
            set_overlay_open(
                ctx.overlay,
                false,
                ctx.core,
                ctx.audio_output,
                ctx.window,
                ctx.session,
            )?;
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
            ctx.overlay
                .set_status_message(format!("[state] saved {}", slot_path.display()));
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
            reset_ephemeral_state(ctx);
            refresh_slot_metadata(ctx.session)?;
            ctx.overlay.focus_slot(slot, false);
            ctx.overlay
                .set_status_message(format!("[state] loaded {}", slot_path.display()));
            Ok(false)
        }
        AppAction::Reset => {
            ctx.core
                .execute(Command::Reset)
                .map_err(|err| format!("Reset failed: {err}"))?;
            reset_ephemeral_state(ctx);
            ctx.overlay.set_status_message("System reset");
            set_overlay_open(
                ctx.overlay,
                false,
                ctx.core,
                ctx.audio_output,
                ctx.window,
                ctx.session,
            )?;
            Ok(false)
        }
        AppAction::Quit => Ok(true),
    }
}

fn reset_ephemeral_state(ctx: &mut AppContext<'_>) {
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
}

fn command_marks_rta_invalidation(command: Command) -> Option<ForbiddenAction> {
    match command {
        Command::StepCpu | Command::StepScanline | Command::StepFrame => {
            Some(ForbiddenAction::FrameStep)
        }
        _ => None,
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
    if let Some(key_code) = crate::input::map_virtual_keycode(key)
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
    apply_gamepad_delta_commands(core, 0, keyboard_bits, nes_core::Player::One)
}

fn is_player_two_slot(player_index: usize) -> bool {
    player_index == 1
}

fn merge_local_input_bits(keyboard_bits: u8, local_gamepad_bits: u8) -> u8 {
    keyboard_bits | local_gamepad_bits
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
    player: nes_core::Player,
) -> Result<(), String> {
    for command in controller_state_delta_for_player(previous_bits, next_bits, player) {
        core.execute(command)
            .map_err(|err| format!("Gamepad command failed: {err}"))?;
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

    let table = build_startup_table(&runtime, &session, &step_mode, rta_manager.as_ref());

    println!("{}", "nes-desktop".with(Color::Cyan).bold());
    println!("\n{table}");
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

    macro_rules! build_ctx {
        () => {
            AppContext {
                core: &mut core,
                session: &mut session,
                session_cheats: &mut session_cheats,
                overlay: &mut overlay,
                rollback_enabled: rollback.is_some(),
                runtime: &runtime,
                audio_output: audio_output.as_ref(),
                time_machine: &mut time_machine,
                rewind_held: &mut rewind_held,
                metrics: &mut metrics,
                keyboard_bits,
                gamepad_bits: &mut gamepad_bits,
                window: &window,
                rta_manager: &mut rta_manager,
                frame_index,
            }
        };
    }
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
                        let mut ctx = build_ctx!();
                        let _ = dispatch_overlay_command(command, &mut ctx, control_flow);
                    }
                    return;
                }
                track_keyboard_bits_for_key(key, pressed, &mut keyboard_bits);
                let mode = KeyboardInputMode {
                    rollback_enabled: rollback.is_some(),
                    rta_enabled: rta_manager.is_some(),
                    rta_calibrate: rta_manager.as_ref().is_some_and(|manager| manager.is_calibrating()),
                };

                match classify_keyboard_input(key, pressed, mode) {
                    KeyboardDecision::ToggleOverlay => {
                        let mut ctx = build_ctx!();
                        let _ = dispatch_app_action(AppAction::ToggleOverlay, &mut ctx, control_flow);
                    }
                    KeyboardDecision::ManualSaveState => {
                        if let Some(action) = slot_action_for_hotkey(true, overlay.selected_slot()) {
                            let _ = {
                                let mut ctx = build_ctx!();
                                dispatch_app_action(action, &mut ctx, control_flow)
                            };
                        }
                    }
                    KeyboardDecision::ManualLoadState => {
                        if let Some(action) = slot_action_for_hotkey(false, overlay.selected_slot()) {
                            let _ = {
                                let mut ctx = build_ctx!();
                                dispatch_app_action(action, &mut ctx, control_flow)
                            };
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
                let mut ctx = build_ctx!();
                if dispatch_app_action(action, &mut ctx, control_flow) {
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
                            if is_player_two_slot(player) {
                                nes_core::Player::Two
                            } else {
                                nes_core::Player::One
                            },
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
            let missed_deadline = match evaluate_frame_deadline(now, next_frame_deadline, TARGET_FRAME_TIME) {
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
                    crate::netplay::compute_local_netplay_bits(gamepad_bits, netplay_local_player);
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
                    if let Some(nonce) = crate::netplay::schedule_netplay_ping(
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
                        if let Err(err) = crate::netplay::handle_netplay_server_message(
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

                        if crate::netplay::should_send_netplay_hash(netplay_hash_check_every, step.frame)
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
                if audio_output.queue_len() >= MAX_AUDIO_QUEUE_CHUNKS {
                    // Fast path: drain samples into a stack array without heap allocation
                    let mut dummy = [0_i16; nes_core::AUDIO_CHUNK_SAMPLES];
                    core.fill_audio_chunk_i16(&mut dummy);
                    metrics.on_audio_queue(audio_output.queue_len(), true);
                } else {
                    let queued = audio_output.queue_samples(core.audio_chunk_i16());
                    metrics.on_audio_queue(audio_output.queue_len(), audio_queue_dropped(queued));
                }
            }

            window.request_redraw();
            *control_flow = ControlFlow::WaitUntil(next_frame_deadline);
        }
        Event::RedrawRequested(_) => {
            let render_start = Instant::now();
            core.fill_framebuffer_rgba(&mut frame_rgba);
            pixels.frame_mut().copy_from_slice(&frame_rgba);
            if overlay.is_open() {
                draw_overlay(
                    pixels.frame_mut(),
                    FRAME_WIDTH,
                    FRAME_HEIGHT,
                    &overlay,
                    session.slot_metadata.iter().map(format_slot_status),
                    session_cheats.entries().iter().map(|entry| OverlayCheatSummary {
                        raw_code: &entry.raw_code,
                        enabled: entry.enabled,
                    }),
                    session_cheats.len(),
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
        nes_core::ppm::encode_ppm(FRAME_WIDTH, FRAME_HEIGHT, rgba).map_err(|e| e.to_string())?
    };
    fs::write(path, bytes).map_err(|err| format!("unable to write '{path}': {err}"))
}

fn build_startup_table(
    runtime: &RuntimeConfig,
    session: &LoadedRomSession,
    step_mode: &StepMode,
    rta_manager: Option<&RtaManager>,
) -> Table {
    let mut table = Table::new();
    table.load_preset(UTF8_FULL);
    table.set_header(vec![
        Cell::new("Property").fg(TableColor::Cyan),
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
        ))
        .fg(TableColor::Green),
    ]);
    if let Some(config_path) = runtime.loaded_config_path.as_ref() {
        table.add_row(vec![
            Cell::new("Config"),
            Cell::new(config_path.display().to_string()).fg(TableColor::Green),
        ]);
    }
    table.add_row(vec![
        Cell::new("Controls"),
        Cell::new(
            "keyboard Z=A, X=B, Enter=Start, RightShift=Select, Arrows=D-pad, R=Rewind, F5=Save Slot, F8=Load Slot, Esc=Menu",
        ).fg(TableColor::Green),
    ]);
    table.add_row(vec![
        Cell::new("Menu"),
        Cell::new(if native_menu_supported() {
            "native menu bar + Esc overlay"
        } else {
            "Esc overlay only on this platform"
        })
        .fg(TableColor::Green),
    ]);
    table.add_row(vec![
        Cell::new("Gamepad"),
        Cell::new("face buttons=A/B, Start/Select, D-pad or left stick").fg(TableColor::Green),
    ]);
    match step_mode {
        StepMode::Frame => {
            table.add_row(vec![
                Cell::new("Step Mode"),
                Cell::new("frame").fg(TableColor::Green),
            ]);
        }
        StepMode::CpuBudget(steps) => {
            table.add_row(vec![
                Cell::new("Step Mode"),
                Cell::new(format!("cpu ({steps} instructions/frame)")).fg(TableColor::Green),
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
            ))
            .fg(TableColor::Green),
        ]);
    }
    if let Some(rta) = rta_manager.as_ref() {
        table.add_row(vec![
            Cell::new("RTA"),
            Cell::new(format!(
                "enabled profile='{}' calibrate={}",
                rta.profile_id(),
                rta.is_calibrating()
            ))
            .fg(TableColor::Green),
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

    table
}

#[cfg(test)]
mod tests {
    #[test]
    fn build_startup_table_creates_expected_table_with_all_options() {
        use super::*;
        use std::path::PathBuf;

        let runtime = RuntimeConfig {
            rom_path: "test.nes".to_string(),
            loaded_config_path: Some(PathBuf::from("config.toml")),
            step_mode: StepMode::Frame,
            audio_enabled: true,
            cheat_codes: vec![],
            rta: None,
            netplay: None,
            window_scale: 2,
            trace_every_frames: 0,
            metrics_enabled: false,
            metrics_every_frames: 0,
            capture: None,
            mcp_enabled: false,
            mcp_bind_addr: "".to_string(),
            #[cfg(feature = "nova")]
            auto_player_enabled: true,
        };

        let session = LoadedRomSession {
            rom_path: PathBuf::from("test.nes"),
            rom_hash: "hash".to_string(),
            info: nes_core::RomLoadInfo {
                mapper_id: 0,
                prg_rom_bytes: 16384,
                reset_pc: 0x8000,
            },
            slot_metadata: vec![],
        };

        let rta_manager = None;
        let table = build_startup_table(&runtime, &session, &runtime.step_mode, rta_manager);

        assert!(table.to_string().contains("ROM Path"));
        assert!(table.to_string().contains("test.nes"));
        assert!(table.to_string().contains("config.toml"));
    }

    use super::{
        FRAME_HEIGHT, FRAME_WIDTH, GAMEPAD_AXIS_THRESHOLD, GamepadSnapshot, NetplayRuntimeStats,
        StepMode, WindowEventDecision, advance_core_for_host_frame, apply_gamepad_delta_commands,
        apply_overlay_keyboard_input, audio_queue_dropped, capture_path_for_frame,
        classify_window_event, connected_gamepad_ids, controller_state_delta_for_player,
        element_state_pressed, format_rom_read_error, gamepad_assignments_changed,
        gamepad_slot_changed, gamepad_snapshot_to_bits, is_player_two_slot, menu_action_enabled,
        merge_local_input_bits, overlay_input_requires_redraw, recommended_input_delay_frames,
        reconcile_core_pause_with_overlay, resync_restored_inputs, rom_picker_supported,
        scaled_window_dimensions, select_active_gamepad_ids, should_capture_frame,
        should_log_rollback, should_resume_after_rewind_hold, should_trace_frame,
        should_update_input_delay, slot_action_for_hotkey, track_keyboard_bits_for_key,
        update_button_bits, validate_action_allowed, write_frame_ppm,
    };
    use gilrs::GamepadId;
    use nes_core::{Button, Command, NesCore};
    use nes_desktop::actions::AppAction;
    use nes_desktop::overlay::OverlayModel;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};
    use winit::dpi::PhysicalSize;
    use winit::event::{
        DeviceId, ElementState, KeyboardInput, ModifiersState, VirtualKeyCode, WindowEvent,
    };

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
    fn map_virtual_keycode_maps_all_supported_keys() {
        assert_eq!(
            crate::input::map_virtual_keycode(VirtualKeyCode::Z),
            Some("KeyZ")
        );
        assert_eq!(
            crate::input::map_virtual_keycode(VirtualKeyCode::X),
            Some("KeyX")
        );
        assert_eq!(
            crate::input::map_virtual_keycode(VirtualKeyCode::Return),
            Some("Enter")
        );
        assert_eq!(
            crate::input::map_virtual_keycode(VirtualKeyCode::RShift),
            Some("ShiftRight")
        );
        assert_eq!(
            crate::input::map_virtual_keycode(VirtualKeyCode::Up),
            Some("ArrowUp")
        );
        assert_eq!(
            crate::input::map_virtual_keycode(VirtualKeyCode::Down),
            Some("ArrowDown")
        );
        assert_eq!(
            crate::input::map_virtual_keycode(VirtualKeyCode::Left),
            Some("ArrowLeft")
        );
        assert_eq!(
            crate::input::map_virtual_keycode(VirtualKeyCode::Right),
            Some("ArrowRight")
        );
        assert_eq!(
            crate::input::map_virtual_keycode(VirtualKeyCode::Escape),
            None
        );
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
    fn selected_slot_hotkeys_target_current_slot() {
        assert_eq!(
            slot_action_for_hotkey(true, 3),
            Some(AppAction::SaveSlot(3))
        );
        assert_eq!(
            slot_action_for_hotkey(false, 3),
            Some(AppAction::LoadSlot(3))
        );
    }

    #[test]
    fn overlay_blocks_gameplay_button_commands_while_open() {
        let mut overlay = OverlayModel::new(5);
        overlay.open();
        let mut keyboard_bits = 0_u8;

        let action = apply_overlay_keyboard_input(
            &mut overlay,
            VirtualKeyCode::Z,
            true,
            0,
            &mut keyboard_bits,
        );

        assert_eq!(action, None);
        assert_eq!(keyboard_bits, 0);
    }

    #[test]
    fn overlay_input_requires_redraw_for_navigation_action_and_text_entry_keys() {
        assert!(overlay_input_requires_redraw(VirtualKeyCode::Up, true));
        assert!(overlay_input_requires_redraw(VirtualKeyCode::Down, true));
        assert!(overlay_input_requires_redraw(VirtualKeyCode::Escape, true));
        assert!(overlay_input_requires_redraw(VirtualKeyCode::Return, true));
        assert!(overlay_input_requires_redraw(VirtualKeyCode::Space, true));
        assert!(overlay_input_requires_redraw(VirtualKeyCode::Delete, true));
        assert!(overlay_input_requires_redraw(VirtualKeyCode::F5, true));
        assert!(overlay_input_requires_redraw(VirtualKeyCode::F8, true));
        assert!(overlay_input_requires_redraw(VirtualKeyCode::Z, true));
        assert!(!overlay_input_requires_redraw(VirtualKeyCode::Up, false));
    }

    #[test]
    fn rollback_disables_stateful_menu_actions() {
        let err = validate_action_allowed(AppAction::OpenRom, true)
            .expect_err("open rom should be blocked during rollback");
        assert!(err.contains("unavailable while netplay/rollback is active"));

        let err = validate_action_allowed(AppAction::OpenCheats, true)
            .expect_err("open cheats should be blocked during rollback");
        assert!(err.contains("unavailable while netplay/rollback is active"));

        let err = validate_action_allowed(AppAction::SaveSlot(2), true)
            .expect_err("save slot should be blocked during rollback");
        assert!(err.contains("unavailable while netplay/rollback is active"));
    }

    #[test]
    fn open_rom_menu_action_requires_platform_picker_support() {
        assert_eq!(
            menu_action_enabled(AppAction::OpenRom, false, false, false),
            rom_picker_supported()
        );
        assert!(menu_action_enabled(
            AppAction::OpenCheats,
            false,
            false,
            false
        ));
        assert!(!menu_action_enabled(
            AppAction::OpenCheats,
            false,
            true,
            false
        ));
        assert!(!menu_action_enabled(
            AppAction::OpenCheats,
            false,
            false,
            true
        ));
    }

    #[test]
    fn sync_native_menu_state_executes_without_panic_in_test_mode() {
        use crate::sync_native_menu_state;
        use nes_desktop::menu::build_native_menu;
        let menu = build_native_menu(3);
        sync_native_menu_state(&menu, false, false, false);
    }

    #[test]
    fn reconcile_core_pause_with_overlay_matches_overlay_visibility() {
        let mut core = NesCore::new();
        core.execute(Command::Pause)
            .expect("pause command should succeed");

        reconcile_core_pause_with_overlay(&mut core, false)
            .expect("closed overlay should force resume");
        assert!(!core.is_paused());

        reconcile_core_pause_with_overlay(&mut core, true)
            .expect("open overlay should force pause");
        assert!(core.is_paused());
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
            nes_core::Player::One,
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
            nes_core::Player::One,
        )
        .expect("releasing one player-1 button should succeed");
        assert_eq!(core.controller_bits(), Button::Right.bit_mask());

        apply_gamepad_delta_commands(
            &mut core,
            0,
            Button::Start.bit_mask(),
            nes_core::Player::Two,
        )
        .expect("applying player-2 gamepad delta should succeed");
        assert_eq!(core.controller2_bits(), Button::Start.bit_mask());
    }

    #[test]
    fn controller_state_delta_emits_press_and_release() {
        let press: Vec<_> = controller_state_delta_for_player(
            0,
            Button::A.bit_mask() | Button::Right.bit_mask(),
            nes_core::Player::One,
        )
        .collect();
        assert_eq!(
            press,
            vec![
                Command::PressButton(Button::A),
                Command::PressButton(Button::Right)
            ]
        );

        let release: Vec<_> = controller_state_delta_for_player(
            Button::A.bit_mask() | Button::B.bit_mask(),
            Button::B.bit_mask(),
            nes_core::Player::One,
        )
        .collect();
        assert_eq!(release, vec![Command::ReleaseButton(Button::A)]);
    }

    #[test]
    fn controller_state_delta_for_player2_uses_player2_commands() {
        let press: Vec<_> =
            controller_state_delta_for_player(0, Button::A.bit_mask(), nes_core::Player::Two)
                .collect();
        assert_eq!(press, vec![Command::PressButton2(Button::A)]);

        let release: Vec<_> =
            controller_state_delta_for_player(Button::Start.bit_mask(), 0, nes_core::Player::Two)
                .collect();
        assert_eq!(release, vec![Command::ReleaseButton2(Button::Start)]);
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
    fn capture_config_helpers_handle_placeholders_and_defaults() {
        assert_eq!(
            capture_path_for_frame("snap-{frame}.ppm", 42),
            "snap-000042.ppm"
        );
        assert_eq!(capture_path_for_frame("snap.ppm", 42), "snap.ppm");
    }

    #[test]
    fn format_rom_read_error_handles_not_found_and_other_errors() {
        let not_found = std::io::Error::from(std::io::ErrorKind::NotFound);
        let msg = format_rom_read_error("bad.nes", &not_found);
        assert!(msg.contains("Could not find the ROM file at"));
        assert!(msg.contains("bad.nes"));
        assert!(msg.contains("homebrew.nes"));

        let other = std::io::Error::from(std::io::ErrorKind::PermissionDenied);
        let msg = format_rom_read_error("bad.nes", &other);
        assert!(msg.contains("Failed to read ROM at"));
        assert!(msg.contains("bad.nes"));
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
