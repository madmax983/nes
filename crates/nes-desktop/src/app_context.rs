use nes_desktop::manual_state::{load_state_file, save_state_file};

use crate::session::{load_rom_session, refresh_slot_metadata, slot_path_for_selection};
use crate::resync_restored_inputs;
use nes_desktop::audio::AudioOutput;
use crate::metrics::PerfMetrics;
use nes_desktop::rta::RtaManager;
use crate::session::LoadedRomSession;
use nes_desktop::session_cheats::SessionCheats;
use nes_core::NesCore;
use nes_rewind::worker::{TimeMachine, TimeMachineConfig};
use winit::window::Window;
use crate::config::RuntimeConfig;
use nes_desktop::overlay::OverlayModel;
use nes_desktop::actions::AppAction;
use winit::event_loop::ControlFlow;
use nes_desktop::overlay::OverlayCommand;
use crate::session::apply_session_cheats;
use nes_core::Command;
use crate::session::window_title;
use nes_desktop::menu::{pick_rom_path, rom_picker_supported};
use nes_desktop::rta::ForbiddenAction;
use std::time::Instant;

pub struct AppContext<'a> {
    pub core: &'a mut NesCore,
    pub session: &'a mut LoadedRomSession,
    pub session_cheats: &'a mut SessionCheats,
    pub overlay: &'a mut OverlayModel,
    pub rollback_enabled: bool,
    pub runtime: &'a RuntimeConfig,
    pub audio_output: Option<&'a AudioOutput>,
    pub time_machine: &'a mut TimeMachine,
    pub rewind_held: &'a mut bool,
    pub metrics: &'a mut PerfMetrics,
    pub keyboard_bits: u8,
    pub gamepad_bits: &'a mut [u8; 2],
    pub window: &'a Window,
    pub rta_manager: &'a mut Option<RtaManager>,
    pub frame_index: u64,
}

pub fn dispatch_app_action(
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


pub fn dispatch_overlay_command(
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


pub fn execute_app_action(action: AppAction, ctx: &mut AppContext<'_>) -> Result<bool, String> {
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
            ctx.overlay
                .set_status_message(format!("[state] loaded {}", slot_path.display()));
            Ok(false)
        }
        AppAction::Reset => {
            ctx.core
                .execute(Command::Reset)
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


pub fn set_overlay_open(
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


pub fn reconcile_core_pause_with_overlay(core: &mut NesCore, overlay_open: bool) -> Result<(), String> {
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


pub fn validate_action_allowed(action: AppAction, rollback_enabled: bool) -> Result<(), String> {
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


pub fn command_marks_rta_invalidation(command: Command) -> Option<ForbiddenAction> {
    match command {
        Command::StepCpu | Command::StepScanline | Command::StepFrame => {
            Some(ForbiddenAction::FrameStep)
        }
        _ => None,
    }
}
