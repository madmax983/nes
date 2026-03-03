use nes_core::{Button, Command};
use nes_desktop::app::map_key_event_to_command;

#[test]
fn keyboard_press_maps_to_controller_command() {
    let cmd = map_key_event_to_command("KeyZ", true).expect("KeyZ should map");
    assert_eq!(cmd.tool_name(), "press_button");
    assert_eq!(cmd.core, Command::PressButton(Button::A));
}

#[test]
fn keyboard_release_maps_to_controller_command() {
    let cmd = map_key_event_to_command("KeyZ", false).expect("KeyZ should map");
    assert_eq!(cmd.tool_name(), "release_button");
    assert_eq!(cmd.core, Command::ReleaseButton(Button::A));
}

#[test]
fn keyboard_maps_remaining_supported_keys() {
    let cases = vec![
        ("KeyX", Command::PressButton(Button::B)),
        ("Enter", Command::PressButton(Button::Start)),
        ("ArrowUp", Command::PressButton(Button::Up)),
        ("ArrowDown", Command::PressButton(Button::Down)),
        ("ArrowLeft", Command::PressButton(Button::Left)),
        ("ArrowRight", Command::PressButton(Button::Right)),
    ];
    for (key, expected) in cases {
        let cmd = map_key_event_to_command(key, true).expect("key should map");
        assert_eq!(cmd.core, expected);
    }
}

#[test]
fn keyboard_unknown_key_returns_none() {
    assert!(map_key_event_to_command("Unknown", true).is_none());
}
