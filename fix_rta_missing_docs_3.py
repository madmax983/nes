with open('crates/nes-desktop/src/rta.rs', 'r') as f:
    content = f.read()

# Let's replace ALL instances of these safely
content = content.replace("    pub name: String,", "    /// The name of the configured split.\n    pub name: String,")
content = content.replace("    pub frame: u64,", "    /// The absolute frame counter indicating when the split triggered.\n    pub frame: u64,")
content = content.replace("    pub elapsed_ms: u128,", "    /// The elapsed time in milliseconds at the split.\n    pub elapsed_ms: u128,")

with open('crates/nes-desktop/src/rta.rs', 'w') as f:
    f.write(content)

with open('crates/nes-desktop/src/menu.rs', 'r') as f:
    content = f.read()

content = content.replace("pub const fn native_menu_supported() -> bool {", "/// Indicates if the host platform supports native OS menus (e.g., Windows/macOS).\npub const fn native_menu_supported() -> bool {")
content = content.replace("pub const fn rom_picker_supported() -> bool {", "/// Indicates if the host platform supports native file picker dialogs.\npub const fn rom_picker_supported() -> bool {")

with open('crates/nes-desktop/src/menu.rs', 'w') as f:
    f.write(content)
