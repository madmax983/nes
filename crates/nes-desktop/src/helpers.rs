use gilrs::GamepadId;
use nes_core::{Command, FRAME_HEIGHT, FRAME_WIDTH};
use nes_desktop::rta::ForbiddenAction;

pub(crate) fn command_marks_rta_invalidation(command: Command) -> Option<ForbiddenAction> {
    match command {
        Command::StepCpu | Command::StepScanline | Command::StepFrame => {
            Some(ForbiddenAction::FrameStep)
        }
        _ => None,
    }
}

pub(crate) fn scaled_window_dimensions(window_scale: u32) -> (f64, f64) {
    (
        f64::from(FRAME_WIDTH as u32 * window_scale),
        f64::from(FRAME_HEIGHT as u32 * window_scale),
    )
}

pub(crate) fn gamepad_assignments_changed(
    next: [Option<GamepadId>; 2],
    current: [Option<GamepadId>; 2],
) -> bool {
    next != current
}

pub(crate) fn gamepad_slot_changed(
    next: [Option<GamepadId>; 2],
    current: [Option<GamepadId>; 2],
    player: usize,
) -> bool {
    next[player] != current[player]
}

pub(crate) fn should_resume_after_rewind_hold(held: bool) -> bool {
    !held
}
