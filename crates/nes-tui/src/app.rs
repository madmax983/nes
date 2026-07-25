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

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::KeyCode;
    use nes_core::{Button, Command};

    #[test]
    fn should_map_keys_to_correct_button_presses() {
        let cases = vec![
            (KeyCode::Char('z'), Button::A),
            (KeyCode::Char('Z'), Button::A),
            (KeyCode::Char('x'), Button::B),
            (KeyCode::Char('X'), Button::B),
            (KeyCode::Enter, Button::Start),
            (KeyCode::Tab, Button::Select),
            (KeyCode::Char('c'), Button::Select),
            (KeyCode::Char('C'), Button::Select),
            (KeyCode::Up, Button::Up),
            (KeyCode::Down, Button::Down),
            (KeyCode::Left, Button::Left),
            (KeyCode::Right, Button::Right),
        ];

        for (key, expected_button) in cases {
            // Test pressed
            let cmd_pressed =
                map_key_event_to_command(key, true).expect("Key should map to a valid button");
            assert_eq!(cmd_pressed.core, Command::PressButton(expected_button));

            // Test released
            let cmd_released =
                map_key_event_to_command(key, false).expect("Key should map to a valid button");
            assert_eq!(cmd_released.core, Command::ReleaseButton(expected_button));
        }
    }

    #[test]
    fn should_return_none_for_unmapped_keys() {
        let unmapped_keys = vec![
            KeyCode::Char('a'),
            KeyCode::Char('1'),
            KeyCode::Esc,
            KeyCode::Backspace,
        ];

        for key in unmapped_keys {
            assert!(map_key_event_to_command(key, true).is_none());
            assert!(map_key_event_to_command(key, false).is_none());
        }
    }

    #[test]
    fn should_return_correct_tool_name_for_bridge_command() {
        let press_cmd = BridgeCommand {
            core: Command::PressButton(Button::A),
        };
        assert_eq!(press_cmd.tool_name(), "press_button");

        let release_cmd = BridgeCommand {
            core: Command::ReleaseButton(Button::B),
        };
        assert_eq!(release_cmd.tool_name(), "release_button");

        let unsupported_cmd = BridgeCommand {
            core: Command::StepFrame,
        };
        assert_eq!(unsupported_cmd.tool_name(), "unsupported");
    }
}
