use nes_core::{Button, Command};

/// A command bridged between the DOM keyboard events and the NES emulator core.
///
/// This struct wraps a [`Command`] targeted for the core to standardize how inputs
/// are structured before being applied.
///
/// ## Examples
///
/// ```
/// use nes_core::{Button, Command};
/// use nes_web::bridge::BridgeCommand;
///
/// let cmd = BridgeCommand { core: Command::PressButton(Button::A) };
/// assert_eq!(cmd.tool_name(), "press_button");
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BridgeCommand {
    /// The underlying emulator core command (e.g. `PressButton(`[`Button::A`]`)$).
    pub core: Command,
}

impl BridgeCommand {
    /// Returns the equivalent tool name string for this command.
    ///
    /// The tool name corresponds to the MCP tool identifiers such as `"press_button"`.
    /// Useful for logging or dispatching via standardized text interfaces.
    ///
    /// ## Examples
    ///
    /// ```
    /// use nes_core::{Button, Command};
    /// use nes_web::bridge::BridgeCommand;
    ///
    /// let cmd = BridgeCommand { core: Command::ReleaseButton(Button::Start) };
    /// assert_eq!(cmd.tool_name(), "release_button");
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

/// Maps a standard DOM `KeyboardEvent.code` to the correct NES button natively.
///
/// Converts web frontend key codes into an actionable [`BridgeCommand`]. Returns
/// `None` if the key does not map to any NES input.
///
/// ## Examples
///
/// ```
/// use nes_core::{Button, Command};
/// use nes_web::bridge::{BridgeCommand, map_dom_key_to_command};
///
/// let cmd = map_dom_key_to_command("Enter", true).unwrap();
/// assert_eq!(cmd.core, Command::PressButton(Button::Start));
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
