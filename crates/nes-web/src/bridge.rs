use nes_core::{Button, Command};

/// Represents an actionable command translated from the browser environment.
///
/// This struct bridges the gap between DOM events (like key presses) and
/// the strict internal [`Command`] types that the emulator expects.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BridgeCommand {
    /// The underlying emulator command.
    pub core: Command,
}

impl BridgeCommand {
    /// Extracts a string identifier for the underlying tool/command.
    ///
    /// ## Examples
    ///
    /// ```rust
    /// use nes_core::{Command, Button};
    /// use nes_web::bridge::BridgeCommand;
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

/// Maps a raw DOM keyboard event code to an emulator bridge command.
///
/// Converts a `KeyboardEvent.code` string (e.g. "ArrowUp") and a boolean
/// representing the key's pressed state into an actionable [`BridgeCommand`].
/// Returns `None` if the key is not mapped to an NES input.
///
/// ## Examples
///
/// ```rust
/// use nes_web::bridge::map_dom_key_to_command;
///
/// // Pressing the 'Z' key maps to the 'A' button.
/// let cmd = map_dom_key_to_command("KeyZ", true).unwrap();
/// assert_eq!(cmd.tool_name(), "press_button");
/// ```
#[must_use]
pub fn map_dom_key_to_command(key_code: &str, pressed: bool) -> Option<BridgeCommand> {
    let button = match key_code {
        "KeyZ" => Button::A,
        "KeyX" => Button::B,
        "Enter" => Button::Start,
        "ShiftRight" => Button::Select,
        "ArrowUp" => Button::Up,
        "ArrowDown" => Button::Down,
        "ArrowLeft" => Button::Left,
        "ArrowRight" => Button::Right,
        _ => return None,
    };

    let core = if pressed {
        Command::PressButton(button)
    } else {
        Command::ReleaseButton(button)
    };

    Some(BridgeCommand { core })
}
