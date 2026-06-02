use nes_core::{Button, Command};

/// Represents a normalized user input action bound for both the UI layer and emulation core.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BridgeCommand {
    /// Action bound for the emulation core (e.g. step frame).
    pub core: Command,
}

impl BridgeCommand {
    /// Retrieves the human-readable name of the specific debug tool window.
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
/// Maps a raw Winit keyboard code to a high-level emulator command.
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
/// Maps a raw Winit keyboard code to a virtual NES controller button.
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
/// Maps a raw Winit keyboard code directly to the NES controller bitmask value.
pub fn map_key_event_to_button_bit(key_code: &str) -> Option<u8> {
    map_key_event_to_button(key_code).map(Button::bit_mask)
}

#[cfg(test)]
mod tests {
    use super::{map_key_event_to_button, map_key_event_to_button_bit};
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
}
