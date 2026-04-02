//! Bridge between `crossterm` terminal events and `nes_core` emulation commands.
//!
//! This module provides the mapping layer that translates raw keyboard inputs
//! from the terminal UI into deterministic controller actions for the NES core.

use crossterm::event::KeyCode;
use nes_core::{Button, Command};

/// A command bridging the TUI frontend to the core emulator.
///
/// Wraps a [`nes_core::Command`] so that the frontend can attach additional
/// metadata or routing logic if necessary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BridgeCommand {
    /// The underlying emulator command.
    pub core: Command,
}

impl BridgeCommand {
    /// Returns a string representation of the tool or action name for this command.
    ///
    /// Useful for logging, debugging, or mapping to an MCP tool schema.
    ///
    /// ## Examples
    ///
    /// ```
    /// use nes_core::{Button, Command};
    /// use nes_tui::app::BridgeCommand;
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

/// Maps a raw `crossterm` keyboard event into a normalized [`BridgeCommand`].
///
/// Returns `None` if the key code does not map to any recognized NES controller button.
///
/// ## Examples
///
/// ```
/// use crossterm::event::KeyCode;
/// use nes_core::{Button, Command};
/// use nes_tui::app::{BridgeCommand, map_key_event_to_command};
///
/// let cmd = map_key_event_to_command(KeyCode::Char('z'), true).unwrap();
/// assert_eq!(cmd.core, Command::PressButton(Button::A));
/// ```
#[must_use]
pub fn map_key_event_to_command(key_code: KeyCode, pressed: bool) -> Option<BridgeCommand> {
    let button = match key_code {
        KeyCode::Char('z') | KeyCode::Char('Z') => Button::A,
        KeyCode::Char('x') | KeyCode::Char('X') => Button::B,
        KeyCode::Enter => Button::Start,
        KeyCode::Tab | KeyCode::Char('c') | KeyCode::Char('C') => Button::Select,
        KeyCode::Up => Button::Up,
        KeyCode::Down => Button::Down,
        KeyCode::Left => Button::Left,
        KeyCode::Right => Button::Right,
        _ => return None,
    };

    let core = if pressed {
        Command::PressButton(button)
    } else {
        Command::ReleaseButton(button)
    };

    Some(BridgeCommand { core })
}
