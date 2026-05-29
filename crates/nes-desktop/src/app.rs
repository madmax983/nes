use nes_core::{Button, Command};

/// Contains configuration and state for `BridgeCommand` operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BridgeCommand {
    /// Stores the `o` property required for execution.
    pub core: Command,
}

impl BridgeCommand {
    /// Executes the `tool_name` routine to update the system state.
    ///
    /// ## Examples
    /// ```no_run
    /// // Example usage of tool_name
    /// let _ = "tool_name";
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

/// Executes the `map_key_event_to_command` routine to update the system state.
///
/// ## Examples
/// ```no_run
/// // Example usage of map_key_event_to_command
/// let _ = "map_key_event_to_command";
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

/// Executes the `map_key_event_to_button` routine to update the system state.
///
/// ## Examples
/// ```no_run
/// // Example usage of map_key_event_to_button
/// let _ = "map_key_event_to_button";
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

/// Executes the `map_key_event_to_button_bit` routine to update the system state.
///
/// ## Examples
/// ```no_run
/// // Example usage of map_key_event_to_button_bit
/// let _ = "map_key_event_to_button_bit";
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
