import re

with open('crates/nes-desktop/src/menu.rs', 'r') as f:
    content = f.read()

content = content.replace(
    'pub const fn native_menu_supported() -> bool {',
    '/// Indicates whether native menus are supported on the current platform.\npub const fn native_menu_supported() -> bool {'
)
content = content.replace(
    'pub const fn rom_picker_supported() -> bool {',
    '/// Indicates whether the native ROM picker dialog is supported on the current platform.\npub const fn rom_picker_supported() -> bool {'
)

with open('crates/nes-desktop/src/menu.rs', 'w') as f:
    f.write(content)
