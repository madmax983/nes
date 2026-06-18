import re

with open('crates/nes-desktop/src/rta.rs', 'r') as f:
    content = f.read()

replacements = {
    "    pub name: String,": "    /// The name of the configured split.\n    pub name: String,",
    "    pub frame: u64,": "    /// The absolute frame counter indicating when the split triggered.\n    pub frame: u64,",
    "    pub elapsed_ms: u128,": "    /// The elapsed time in milliseconds at the split.\n    pub elapsed_ms: u128,"
}

for pat, repl in replacements.items():
    parts = content.split(pat, 1)
    if len(parts) > 1:
        content = parts[0] + repl + parts[1]

with open('crates/nes-desktop/src/rta.rs', 'w') as f:
    f.write(content)

with open('crates/nes-desktop/src/menu.rs', 'r') as f:
    content = f.read()

content = content.replace("pub const fn native_menu_supported() -> bool {", "/// Indicates if the host platform supports native OS menus (e.g., Windows/macOS).\npub const fn native_menu_supported() -> bool {", 1)
content = content.replace("pub const fn rom_picker_supported() -> bool {", "/// Indicates if the host platform supports native file picker dialogs.\npub const fn rom_picker_supported() -> bool {", 1)

with open('crates/nes-desktop/src/menu.rs', 'w') as f:
    f.write(content)
