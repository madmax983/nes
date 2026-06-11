use crate::config::RuntimeConfig;
use crate::main_helpers::{reconcile_core_pause_with_overlay, set_overlay_open};
use crate::metrics::PerfMetrics;
use crate::session::apply_session_cheats;
use crate::session::{
    LoadedRomSession, load_rom_session, refresh_slot_metadata, slot_path_for_selection,
    window_title,
};
use nes_core::{Command, NesCore};
use nes_desktop::actions::AppAction;
use nes_desktop::audio::AudioOutput;
use nes_desktop::manual_state::{load_state_file, save_state_file};
use nes_desktop::menu::{pick_rom_path, rom_picker_supported};
use nes_desktop::overlay::OverlayModel;
use nes_desktop::rta::{ForbiddenAction, RtaManager};
use nes_desktop::session_cheats::SessionCheats;
use nes_rewind::worker::{TimeMachine, TimeMachineConfig};
use std::time::Instant;
use winit::event_loop::ControlFlow;
use winit::window::Window;

pub(crate) struct AppContext<'a> {
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

pub(crate) fn validate_action_allowed(
    action: AppAction,
    rollback_enabled: bool,
) -> Result<(), String> {
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

pub(crate) fn dispatch_app_action(
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

pub(crate) fn resync_restored_inputs(
    core: &mut NesCore,
    keyboard_bits: u8,
    gamepad_bits: &mut [u8; 2],
) -> Result<(), String> {
    crate::main_helpers::release_all_buttons(core);
    *gamepad_bits = [0; 2];
    crate::main_helpers::apply_gamepad_delta_commands(core, 0, keyboard_bits, nes_core::Player::One)
}

pub(crate) fn execute_app_action(
    action: AppAction,
    ctx: &mut AppContext<'_>,
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

pub(crate) fn reset_ephemeral_state(ctx: &mut AppContext<'_>) {
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
