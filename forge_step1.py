content = open('crates/nes-desktop/src/main.rs').read()
search = """fn overlay_input_requires_redraw(key: VirtualKeyCode, pressed: bool) -> bool {
    pressed
        && (matches!(
            key,
            VirtualKeyCode::Up
                | VirtualKeyCode::Down
                | VirtualKeyCode::Escape
                | VirtualKeyCode::Return
                | VirtualKeyCode::Space
                | VirtualKeyCode::Delete
                | VirtualKeyCode::Back
                | VirtualKeyCode::F5
                | VirtualKeyCode::F8
        ) || matches!(
            key,
            VirtualKeyCode::A
                | VirtualKeyCode::E
                | VirtualKeyCode::G
                | VirtualKeyCode::I
                | VirtualKeyCode::K
                | VirtualKeyCode::L
                | VirtualKeyCode::N
                | VirtualKeyCode::O
                | VirtualKeyCode::P
                | VirtualKeyCode::S
                | VirtualKeyCode::T
                | VirtualKeyCode::U
                | VirtualKeyCode::V
                | VirtualKeyCode::X
                | VirtualKeyCode::Y
                | VirtualKeyCode::Z
        ))
}"""
replace = """fn overlay_input_requires_redraw(key: VirtualKeyCode, pressed: bool) -> bool {
    pressed
        && matches!(
            key,
            VirtualKeyCode::Up
                | VirtualKeyCode::Down
                | VirtualKeyCode::Escape
                | VirtualKeyCode::Return
                | VirtualKeyCode::Space
                | VirtualKeyCode::Delete
                | VirtualKeyCode::Back
                | VirtualKeyCode::F5
                | VirtualKeyCode::F8
                | VirtualKeyCode::A
                | VirtualKeyCode::E
                | VirtualKeyCode::G
                | VirtualKeyCode::I
                | VirtualKeyCode::K
                | VirtualKeyCode::L
                | VirtualKeyCode::N
                | VirtualKeyCode::O
                | VirtualKeyCode::P
                | VirtualKeyCode::S
                | VirtualKeyCode::T
                | VirtualKeyCode::U
                | VirtualKeyCode::V
                | VirtualKeyCode::X
                | VirtualKeyCode::Y
                | VirtualKeyCode::Z
        )
}"""
open('crates/nes-desktop/src/main.rs', 'w').write(content.replace(search, replace, 1))
print(f"Replaced? {search in content}")
