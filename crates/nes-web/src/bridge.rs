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
