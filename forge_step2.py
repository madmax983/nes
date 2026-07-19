content = open('crates/nes-desktop/src/input.rs').read()
search = """    if key == VirtualKeyCode::Escape && pressed {
        return KeyboardDecision::ToggleOverlay;
    }
    if pressed && key == VirtualKeyCode::F5 {
        return KeyboardDecision::ManualSaveState;
    }
    if pressed && key == VirtualKeyCode::F8 {
        return KeyboardDecision::ManualLoadState;
    }
    if key == VirtualKeyCode::R {
        return KeyboardDecision::SetRewindHeld(pressed);
    }
    if mode.rta_enabled && pressed && key == VirtualKeyCode::F9 {
        return KeyboardDecision::RtaManualSplit;
    }
    if mode.rta_enabled && mode.rta_calibrate && pressed && key == VirtualKeyCode::F10 {
        return KeyboardDecision::RtaFinish;
    }"""
replace = """    match (key, pressed) {
        (VirtualKeyCode::Escape, true) => return KeyboardDecision::ToggleOverlay,
        (VirtualKeyCode::F5, true) => return KeyboardDecision::ManualSaveState,
        (VirtualKeyCode::F8, true) => return KeyboardDecision::ManualLoadState,
        (VirtualKeyCode::R, _) => return KeyboardDecision::SetRewindHeld(pressed),
        (VirtualKeyCode::F9, true) if mode.rta_enabled => return KeyboardDecision::RtaManualSplit,
        (VirtualKeyCode::F10, true) if mode.rta_enabled && mode.rta_calibrate => return KeyboardDecision::RtaFinish,
        _ => {}
    }"""
open('crates/nes-desktop/src/input.rs', 'w').write(content.replace(search, replace, 1))
print(f"Replaced? {search in content}")
