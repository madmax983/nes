use nes_web::bridge::map_dom_key_to_command;

#[test]
fn dom_key_maps_to_press_button_command() {
    let cmd = map_dom_key_to_command("KeyX", true).unwrap();
    assert_eq!(cmd.tool_name(), "press_button");
}
