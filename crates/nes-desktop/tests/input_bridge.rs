use nes_desktop::app::map_key_event_to_command;

#[test]
fn keyboard_press_maps_to_controller_command() {
    let cmd = map_key_event_to_command("KeyZ", true).unwrap();
    assert_eq!(cmd.tool_name(), "press_button");
}
