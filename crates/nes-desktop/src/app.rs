//! Input Bridge mappings for the desktop runtime.
//!
//! This module provides utilities to map raw keyboard event codes from the desktop
//! environment (like `winit`) into actions that the `NesCore` understands.

use nes_core::{Button, Command};

/// A command bridging a desktop input to an emulator action.
///
/// This serves as an intermediate representation before applying the command
/// directly to the `NesCore`, enabling logging, macros, or network play handling.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BridgeCommand {
    /// The underlying emulator command.
    pub core: Command,
}

impl BridgeCommand {
    /// Returns a string identifier for the tool or command type.
    ///
    /// Useful for logging or matching command types without unwrapping inner values.
    ///
    /// ## Examples
    ///
    /// ```rust
    /// use nes_core::{Button, Command};
    /// use nes_desktop::app::BridgeCommand;
    ///
    /// let cmd = BridgeCommand { core: Command::PressButton(Button::A) };
    /// assert_eq!(cmd.tool_name(), "press_button");
    /// ```
    #[must_use]
    pub fn tool_name(self) -> &'static str {
        match self.core {
            Command::PressButton(_) => "press_button",
            Command::ReleaseButton(_) => "release_button",
            _ => "unsupported",
        }
    }
}

/// Maps a string key code to an optional emulator command based on the press state.
///
/// This translates a physical key (e.g., `"KeyZ"`) and whether it was pressed or
/// released into a `BridgeCommand`. Returns `None` if the key is not mapped to
/// any NES button.
///
/// ## Examples
///
/// ```rust
/// use nes_core::{Button, Command};
/// use nes_desktop::app::{BridgeCommand, map_key_event_to_command};
///
/// let cmd = map_key_event_to_command("Enter", true).unwrap();
/// assert_eq!(cmd.core, Command::PressButton(Button::Start));
/// ```
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

/// Maps a raw key code string to an NES `Button`.
///
/// This defines the hardcoded keyboard layout for the emulator.
/// For example, `"KeyZ"` maps to `Button::A`. Returns `None` if
/// the key has no mapping.
///
/// ## Examples
///
/// ```rust
/// use nes_core::Button;
/// use nes_desktop::app::map_key_event_to_button;
///
/// assert_eq!(map_key_event_to_button("KeyX"), Some(Button::B));
/// assert_eq!(map_key_event_to_button("ShiftLeft"), Some(Button::Select));
/// assert_eq!(map_key_event_to_button("Q"), None);
/// ```
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

/// Maps a raw key code string directly to the bitmask of its corresponding NES button.
///
/// This is a convenience function useful when constructing raw input bytes for netplay
/// or TAS recording.
///
/// ## Examples
///
/// ```rust
/// use nes_core::Button;
/// use nes_desktop::app::map_key_event_to_button_bit;
///
/// assert_eq!(map_key_event_to_button_bit("KeyZ"), Some(Button::A.bit_mask()));
/// ```
#[must_use]
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
