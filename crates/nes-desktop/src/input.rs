use nes_core::Command;
use nes_desktop::app::{map_key_event_to_button_bit, map_key_event_to_command};
use nes_desktop::overlay::{OverlayCommand, OverlayModel};
use std::time::{Duration, Instant};
use winit::event::{ElementState, VirtualKeyCode, WindowEvent};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum WindowEventDecision {
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum KeyboardDecision {
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
pub(crate) enum FrameDecision {
    WaitUntil(Instant),
    Step {
        missed_deadline: bool,
        next_deadline: Instant,
    },
}

#[derive(Debug, Clone, Copy)]
pub struct KeyboardInputMode {
    pub rollback_enabled: bool,
    pub rta_enabled: bool,
    pub rta_calibrate: bool,
}

#[must_use]
pub(crate) fn classify_window_event(event: &WindowEvent<'_>) -> WindowEventDecision {
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

pub(crate) fn apply_overlay_keyboard_input(
    overlay: &mut OverlayModel,
    key: VirtualKeyCode,
    pressed: bool,
    cheat_count: usize,
    _keyboard_bits: &mut u8,
) -> Option<OverlayCommand> {
    overlay.handle_key(key, pressed, cheat_count)
}

pub(crate) fn overlay_input_requires_redraw(key: VirtualKeyCode, pressed: bool) -> bool {
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

pub(crate) fn map_virtual_keycode(key: VirtualKeyCode) -> Option<&'static str> {
    match key {
        VirtualKeyCode::Up => Some("ArrowUp"),
        VirtualKeyCode::Down => Some("ArrowDown"),
        VirtualKeyCode::Left => Some("ArrowLeft"),
        VirtualKeyCode::Right => Some("ArrowRight"),
        VirtualKeyCode::Z => Some("KeyZ"),
        VirtualKeyCode::X => Some("KeyX"),
        VirtualKeyCode::S => Some("KeyS"),
        VirtualKeyCode::A => Some("KeyA"),
        VirtualKeyCode::Return => Some("Enter"),
        VirtualKeyCode::RShift => Some("ShiftRight"),
        _ => None,
    }
}

#[must_use]
pub(crate) fn classify_keyboard_input(
    key: VirtualKeyCode,
    pressed: bool,
    mode: KeyboardInputMode,
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
    if mode.rta_enabled && pressed && key == VirtualKeyCode::F9 {
        return KeyboardDecision::RtaManualSplit;
    }
    if mode.rta_enabled && mode.rta_calibrate && pressed && key == VirtualKeyCode::F10 {
        return KeyboardDecision::RtaFinish;
    }

    let Some(key_code) = map_virtual_keycode(key) else {
        return KeyboardDecision::Noop;
    };

    if mode.rollback_enabled {
        let Some(mask) = map_key_event_to_button_bit(key_code) else {
            return KeyboardDecision::Noop;
        };
        KeyboardDecision::UpdateKeyboardBits { mask, pressed }
    } else {
        let Some(mapped) = map_key_event_to_command(key_code, pressed) else {
            return KeyboardDecision::Noop;
        };
        KeyboardDecision::ExecuteCore(mapped.core)
    }
}

#[must_use]
pub(crate) fn evaluate_frame_deadline(
    now: Instant,
    next_frame_deadline: Instant,
    target_frame_time: Duration,
) -> FrameDecision {
    if now < next_frame_deadline {
        FrameDecision::WaitUntil(next_frame_deadline)
    } else {
        FrameDecision::Step {
            missed_deadline: now > next_frame_deadline,
            next_deadline: now + target_frame_time,
        }
    }
}

#[must_use]
pub(crate) fn element_state_pressed(state: ElementState) -> bool {
    state == ElementState::Pressed
}

#[cfg(test)]
mod tests {
    use super::*;
    use nes_core::Button;
    use nes_desktop::overlay::OverlayModel;
    use winit::dpi::PhysicalSize;
    use winit::event::{ElementState, KeyboardInput, WindowEvent};

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
    fn map_virtual_keycode_maps_all_keys() {
        assert_eq!(map_virtual_keycode(VirtualKeyCode::Up), Some("ArrowUp"));
        assert_eq!(map_virtual_keycode(VirtualKeyCode::Down), Some("ArrowDown"));
        assert_eq!(map_virtual_keycode(VirtualKeyCode::Left), Some("ArrowLeft"));
        assert_eq!(
            map_virtual_keycode(VirtualKeyCode::Right),
            Some("ArrowRight")
        );
        assert_eq!(map_virtual_keycode(VirtualKeyCode::Z), Some("KeyZ"));
        assert_eq!(map_virtual_keycode(VirtualKeyCode::X), Some("KeyX"));
        assert_eq!(map_virtual_keycode(VirtualKeyCode::S), Some("KeyS"));
        assert_eq!(map_virtual_keycode(VirtualKeyCode::A), Some("KeyA"));
        assert_eq!(map_virtual_keycode(VirtualKeyCode::Return), Some("Enter"));
        assert_eq!(
            map_virtual_keycode(VirtualKeyCode::RShift),
            Some("ShiftRight")
        );
        assert_eq!(map_virtual_keycode(VirtualKeyCode::F5), None);
        assert_eq!(map_virtual_keycode(VirtualKeyCode::Escape), None);
    }

    #[test]
    fn classify_window_event_maps_window_variants_to_decisions() {
        assert_eq!(
            classify_window_event(&WindowEvent::CloseRequested),
            WindowEventDecision::CloseRequested
        );

        let key_event = WindowEvent::KeyboardInput {
            device_id: unsafe { winit::event::DeviceId::dummy() },
            #[allow(deprecated)]
            input: KeyboardInput {
                scancode: 0,
                state: ElementState::Pressed,
                virtual_keycode: Some(VirtualKeyCode::Z),
                modifiers: Default::default(),
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
    fn classify_keyboard_input_rta_calibrate_conditions() {
        let base_mode = KeyboardInputMode {
            rollback_enabled: false,
            rta_enabled: false,
            rta_calibrate: false,
        };

        // All conditions met
        assert_eq!(
            classify_keyboard_input(
                VirtualKeyCode::F10,
                true,
                KeyboardInputMode {
                    rta_enabled: true,
                    rta_calibrate: true,
                    ..base_mode
                }
            ),
            KeyboardDecision::RtaFinish
        );

        // rta_enabled is false
        assert_eq!(
            classify_keyboard_input(
                VirtualKeyCode::F10,
                true,
                KeyboardInputMode {
                    rta_enabled: false,
                    rta_calibrate: true,
                    ..base_mode
                }
            ),
            KeyboardDecision::Noop
        );

        // rta_calibrate is false
        assert_eq!(
            classify_keyboard_input(
                VirtualKeyCode::F10,
                true,
                KeyboardInputMode {
                    rta_enabled: true,
                    rta_calibrate: false,
                    ..base_mode
                }
            ),
            KeyboardDecision::Noop
        );

        // pressed is false
        assert_eq!(
            classify_keyboard_input(
                VirtualKeyCode::F10,
                false,
                KeyboardInputMode {
                    rta_enabled: true,
                    rta_calibrate: true,
                    ..base_mode
                }
            ),
            KeyboardDecision::Noop
        );
    }

    #[test]
    fn classify_keyboard_input_covers_exit_rewind_rollback_and_core_paths() {
        let base_mode = KeyboardInputMode {
            rollback_enabled: false,
            rta_enabled: false,
            rta_calibrate: false,
        };

        assert_eq!(
            classify_keyboard_input(VirtualKeyCode::Escape, true, base_mode),
            KeyboardDecision::ToggleOverlay
        );
        assert_eq!(
            classify_keyboard_input(VirtualKeyCode::F5, true, base_mode),
            KeyboardDecision::ManualSaveState
        );
        assert_eq!(
            classify_keyboard_input(VirtualKeyCode::F8, true, base_mode),
            KeyboardDecision::ManualLoadState
        );
        assert_eq!(
            classify_keyboard_input(VirtualKeyCode::R, true, base_mode),
            KeyboardDecision::SetRewindHeld(true)
        );
        assert_eq!(
            classify_keyboard_input(
                VirtualKeyCode::R,
                false,
                KeyboardInputMode {
                    rollback_enabled: true,
                    ..base_mode
                }
            ),
            KeyboardDecision::SetRewindHeld(false)
        );
        assert_eq!(
            classify_keyboard_input(
                VirtualKeyCode::Z,
                true,
                KeyboardInputMode {
                    rollback_enabled: true,
                    ..base_mode
                }
            ),
            KeyboardDecision::UpdateKeyboardBits {
                mask: Button::A.bit_mask(),
                pressed: true
            }
        );
        assert_eq!(
            classify_keyboard_input(VirtualKeyCode::X, false, base_mode),
            KeyboardDecision::ExecuteCore(Command::ReleaseButton(Button::B))
        );
        assert_eq!(
            classify_keyboard_input(VirtualKeyCode::Escape, false, base_mode),
            KeyboardDecision::Noop
        );
        assert_eq!(
            classify_keyboard_input(
                VirtualKeyCode::F9,
                true,
                KeyboardInputMode {
                    rta_enabled: true,
                    ..base_mode
                }
            ),
            KeyboardDecision::RtaManualSplit
        );
        assert_eq!(
            classify_keyboard_input(
                VirtualKeyCode::F10,
                true,
                KeyboardInputMode {
                    rta_enabled: true,
                    rta_calibrate: true,
                    ..base_mode
                }
            ),
            KeyboardDecision::RtaFinish
        );
        assert_eq!(
            classify_keyboard_input(VirtualKeyCode::F5, false, base_mode),
            KeyboardDecision::Noop
        );
        assert_eq!(
            classify_keyboard_input(
                VirtualKeyCode::F5,
                true,
                KeyboardInputMode {
                    rollback_enabled: true,
                    ..base_mode
                }
            ),
            KeyboardDecision::ManualSaveState
        );
        assert_eq!(
            classify_keyboard_input(
                VirtualKeyCode::F5,
                false,
                KeyboardInputMode {
                    rollback_enabled: true,
                    ..base_mode
                }
            ),
            KeyboardDecision::Noop
        );
        assert_eq!(
            classify_keyboard_input(
                VirtualKeyCode::F5,
                false,
                KeyboardInputMode {
                    rollback_enabled: false,
                    ..base_mode
                }
            ),
            KeyboardDecision::Noop
        );
    }

    #[test]
    fn evaluate_frame_deadline_classifies_wait_and_step_cases() {
        let now = Instant::now();
        let target_frame_time = Duration::from_nanos(16_639_264);
        let future = now + Duration::from_millis(2);
        match evaluate_frame_deadline(now, future, target_frame_time) {
            FrameDecision::WaitUntil(deadline) => assert_eq!(deadline, future),
            FrameDecision::Step { .. } => panic!("expected wait branch"),
        }

        match evaluate_frame_deadline(now, now, target_frame_time) {
            FrameDecision::Step {
                missed_deadline,
                next_deadline,
            } => {
                assert!(!missed_deadline);
                assert_eq!(next_deadline, now + target_frame_time);
            }
            FrameDecision::WaitUntil(_) => panic!("expected step branch"),
        }

        let past = now - Duration::from_millis(1);
        match evaluate_frame_deadline(now, past, target_frame_time) {
            FrameDecision::Step {
                missed_deadline,
                next_deadline,
            } => {
                assert!(missed_deadline);
                assert_eq!(next_deadline, now + target_frame_time);
            }
            FrameDecision::WaitUntil(_) => panic!("expected step branch"),
        }
    }
}
