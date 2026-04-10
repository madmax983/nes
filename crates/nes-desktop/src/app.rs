use nes_core::{Button, Command};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BridgeCommand {
    pub core: Command,
}

impl BridgeCommand {
    #[must_use]
    pub fn tool_name(self) -> &'static str {
        match self.core {
            Command::PressButton(_) => "press_button",
            Command::ReleaseButton(_) => "release_button",
            _ => "unsupported",
        }
    }
}

#[must_use]
pub fn map_key_event_to_command(key_code: &str, pressed: bool) -> Option<BridgeCommand> {
    let button = map_key_event_to_button(key_code)?;

    let core = if pressed {
        Command::PressButton(button)
    } else {
        Command::ReleaseButton(button)
    };

    Some(BridgeCommand { core })
}

#[must_use]
pub fn map_key_event_to_button(key_code: &str) -> Option<Button> {
    match key_code {
        "KeyZ" => Some(Button::A),
        "KeyX" => Some(Button::B),
        "Enter" => Some(Button::Start),
        "ShiftLeft" | "ShiftRight" => Some(Button::Select),
        "ArrowUp" => Some(Button::Up),
        "ArrowDown" => Some(Button::Down),
        "ArrowLeft" => Some(Button::Left),
        "ArrowRight" => Some(Button::Right),
        _ => None,
    }
}

#[must_use]
pub fn map_key_event_to_button_bit(key_code: &str) -> Option<u8> {
    map_key_event_to_button(key_code).map(Button::bit_mask)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyboardDecision {
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
pub struct KeyboardInputMode {
    pub rollback_enabled: bool,
    pub rta_enabled: bool,
    pub rta_calibrate: bool,
}

pub fn classify_keyboard_input(
    key: winit::event::VirtualKeyCode,
    pressed: bool,
    mode: KeyboardInputMode,
) -> KeyboardDecision {
    if key == winit::event::VirtualKeyCode::Escape && pressed {
        return KeyboardDecision::ToggleOverlay;
    }
    if pressed && key == winit::event::VirtualKeyCode::F5 {
        return KeyboardDecision::ManualSaveState;
    }
    if pressed && key == winit::event::VirtualKeyCode::F8 {
        return KeyboardDecision::ManualLoadState;
    }
    if key == winit::event::VirtualKeyCode::R {
        return KeyboardDecision::SetRewindHeld(pressed);
    }
    if mode.rta_enabled && pressed && key == winit::event::VirtualKeyCode::F9 {
        return KeyboardDecision::RtaManualSplit;
    }
    if mode.rta_enabled && mode.rta_calibrate && pressed && key == winit::event::VirtualKeyCode::F10 {
        return KeyboardDecision::RtaFinish;
    }

    let Some(key_code) = map_virtual_keycode(key) else {
        return KeyboardDecision::Noop;
    };

    if mode.rollback_enabled {
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

pub fn map_virtual_keycode(key: winit::event::VirtualKeyCode) -> Option<&'static str> {
    match key {
        winit::event::VirtualKeyCode::Z => Some("KeyZ"),
        winit::event::VirtualKeyCode::X => Some("KeyX"),
        winit::event::VirtualKeyCode::Return => Some("Enter"),
        winit::event::VirtualKeyCode::RShift => Some("ShiftRight"),
        winit::event::VirtualKeyCode::Up => Some("ArrowUp"),
        winit::event::VirtualKeyCode::Down => Some("ArrowDown"),
        winit::event::VirtualKeyCode::Left => Some("ArrowLeft"),
        winit::event::VirtualKeyCode::Right => Some("ArrowRight"),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::{
        classify_keyboard_input, map_key_event_to_button, map_key_event_to_button_bit,
        map_virtual_keycode, KeyboardDecision, KeyboardInputMode,
    };
    use winit::event::VirtualKeyCode;
    use nes_core::Button;

    #[test]
    fn key_mapping_supports_both_shift_keys_for_select() {
        assert_eq!(map_key_event_to_button("ShiftLeft"), Some(Button::Select));
        assert_eq!(map_key_event_to_button("ShiftRight"), Some(Button::Select));
    }

    #[test]
    fn key_bit_mapping_returns_expected_mask() {
        assert_eq!(
            map_key_event_to_button_bit("KeyZ"),
            Some(Button::A.bit_mask())
        );
        assert_eq!(
            map_key_event_to_button_bit("ArrowRight"),
            Some(Button::Right.bit_mask())
        );
        assert_eq!(map_key_event_to_button_bit("Unknown"), None);
    }

    use nes_core::Command;

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
                mask: nes_core::Button::A.bit_mask(),
                pressed: true
            }
        );
        assert_eq!(
            classify_keyboard_input(VirtualKeyCode::X, false, base_mode),
            KeyboardDecision::ExecuteCore(Command::ReleaseButton(nes_core::Button::B))
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
    }
}
