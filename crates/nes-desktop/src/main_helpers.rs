use crate::gamepad::controller_state_delta_for_player;
use crate::session::LoadedRomSession;
use crate::session::window_title;
use nes_core::{Command, NesCore};
use nes_desktop::actions::AppAction;
use nes_desktop::audio::AudioOutput;
use nes_desktop::overlay::OverlayModel;
use winit::window::Window;

pub(crate) fn reconcile_core_pause_with_overlay(
    core: &mut NesCore,
    overlay_open: bool,
) -> Result<(), String> {
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

pub(crate) fn set_overlay_open(
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

pub(crate) fn slot_action_for_hotkey(is_save: bool, selected_slot: u8) -> Option<AppAction> {
    if !(1..=5).contains(&selected_slot) {
        return None;
    }
    Some(if is_save {
        AppAction::SaveSlot(selected_slot)
    } else {
        AppAction::LoadSlot(selected_slot)
    })
}

pub(crate) const CONTROLLER_BUTTONS: [nes_core::Button; 8] = [
    nes_core::Button::A,
    nes_core::Button::B,
    nes_core::Button::Select,
    nes_core::Button::Start,
    nes_core::Button::Up,
    nes_core::Button::Down,
    nes_core::Button::Left,
    nes_core::Button::Right,
];

pub(crate) fn release_all_buttons(core: &mut NesCore) {
    for &button in &CONTROLLER_BUTTONS {
        let _ = core.execute(Command::ReleaseButton(button));
        let _ = core.execute(Command::ReleaseButton2(button));
    }
}

pub(crate) fn apply_gamepad_delta_commands(
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
