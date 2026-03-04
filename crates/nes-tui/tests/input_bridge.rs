use crossterm::event::KeyCode;
use nes_core::{Button, Command};
use nes_tui::app::map_key_event_to_command;

#[test]
fn keyboard_press_maps_to_controller_command() {
    let cmd = map_key_event_to_command(KeyCode::Char('z'), true)
        .expect("z should map to a press button command");
    assert_eq!(cmd.tool_name(), "press_button");
    assert_eq!(cmd.core, Command::PressButton(Button::A));
}

#[test]
fn keyboard_release_maps_to_controller_command() {
    let cmd = map_key_event_to_command(KeyCode::Right, false)
        .expect("right arrow should map to a release button command");
    assert_eq!(cmd.tool_name(), "release_button");
    assert_eq!(cmd.core, Command::ReleaseButton(Button::Right));
}

#[test]
fn keyboard_maps_remaining_controller_keys() {
    let cases = vec![
        (KeyCode::Char('x'), Command::PressButton(Button::B)),
        (KeyCode::Enter, Command::PressButton(Button::Start)),
        (KeyCode::Tab, Command::PressButton(Button::Select)),
        (KeyCode::Char('c'), Command::PressButton(Button::Select)),
        (KeyCode::Up, Command::PressButton(Button::Up)),
        (KeyCode::Down, Command::PressButton(Button::Down)),
        (KeyCode::Left, Command::PressButton(Button::Left)),
    ];

    for (key, expected) in cases {
        let cmd = map_key_event_to_command(key, true)
            .expect("mapped key should produce a controller command");
        assert_eq!(cmd.core, expected);
    }
}
