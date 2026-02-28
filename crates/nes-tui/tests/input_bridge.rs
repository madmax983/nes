use crossterm::event::KeyCode;
use nes_tui::app::map_key_event_to_command;

#[test]
fn keyboard_press_maps_to_controller_command() {
    let cmd = map_key_event_to_command(KeyCode::Char('z'), true)
        .expect("z should map to a press button command");
    assert_eq!(cmd.tool_name(), "press_button");
}

#[test]
fn keyboard_release_maps_to_controller_command() {
    let cmd = map_key_event_to_command(KeyCode::Right, false)
        .expect("right arrow should map to a release button command");
    assert_eq!(cmd.tool_name(), "release_button");
}
