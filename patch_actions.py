import re

with open('crates/nes-desktop/src/actions.rs', 'r') as f:
    content = f.read()

content = re.sub(r'(\s+)ToggleOverlay,', r'\1/// Toggle the pause overlay.\1ToggleOverlay,', content)
content = re.sub(r'(\s+)Resume,', r'\1/// Resume the emulation.\1Resume,', content)
content = re.sub(r'(\s+)OpenRom,', r'\1/// Open a new ROM.\1OpenRom,', content)
content = re.sub(r'(\s+)OpenCheats,', r'\1/// Open the cheats configuration.\1OpenCheats,', content)
content = re.sub(r'(\s+)SaveSlot\(u8\),', r'\1/// Save state to the given slot.\1SaveSlot(u8),', content)
content = re.sub(r'(\s+)LoadSlot\(u8\),', r'\1/// Load state from the given slot.\1LoadSlot(u8),', content)
content = re.sub(r'(\s+)Reset,', r'\1/// Reset the emulator.\1Reset,', content)
content = re.sub(r'(\s+)Quit,', r'\1/// Quit the application.\1Quit,', content)

with open('crates/nes-desktop/src/actions.rs', 'w') as f:
    f.write(content)
