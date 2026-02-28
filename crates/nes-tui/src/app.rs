use crossterm::event::KeyCode;
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
