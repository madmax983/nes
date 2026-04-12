import re

with open("crates/nes-desktop/src/input.rs", "r") as f:
    content = f.read()

target = """            #[allow(deprecated)]
            input: KeyboardInput {
                scancode: 0,
                state: ElementState::Pressed,
                virtual_keycode: Some(VirtualKeyCode::Z),
            },"""
replace = """            #[allow(deprecated)]
            input: KeyboardInput {
                scancode: 0,
                state: ElementState::Pressed,
                virtual_keycode: Some(VirtualKeyCode::Z),
                modifiers: Default::default(),
            },"""
content = content.replace(target, replace)

with open("crates/nes-desktop/src/input.rs", "w") as f:
    f.write(content)

with open("crates/nes-desktop/src/main.rs", "r") as f:
    content = f.read()

target2 = "use super::KeyboardInputMode;\n\n    #[test]"
replace2 = "#[test]"
content = content.replace(target2, replace2)

target3 = "FRAME_HEIGHT, FRAME_WIDTH, GAMEPAD_AXIS_THRESHOLD, GamepadSnapshot, KeyboardDecision,\n        NetplayRuntimeStats, StepMode, WindowEventDecision, advance_core_for_host_frame,"
replace3 = "FRAME_HEIGHT, FRAME_WIDTH, GAMEPAD_AXIS_THRESHOLD, GamepadSnapshot,\n        NetplayRuntimeStats, StepMode, WindowEventDecision, advance_core_for_host_frame,"
content = content.replace(target3, replace3)

target4 = "capture_path_for_frame, classify_keyboard_input, classify_window_event,"
replace4 = "capture_path_for_frame, classify_window_event,"
content = content.replace(target4, replace4)

with open("crates/nes-desktop/src/main.rs", "w") as f:
    f.write(content)
