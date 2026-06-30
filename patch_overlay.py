import re

with open('crates/nes-desktop/src/overlay.rs', 'r') as f:
    content = f.read()

# Add docs for OverlayPanel
content = re.sub(r'(\s+)MainMenu,', r'\1/// The primary pause menu.\1MainMenu,', content)
content = re.sub(r'(\s+)Cheats,', r'\1/// The cheats configuration screen.\1Cheats,', content)

# Add docs for MainMenuSelection
content = re.sub(r'(\s+)Resume,', r'\1/// Resume the current emulation session.\1Resume,', content)
content = re.sub(r'(\s+)OpenRom,', r'\1/// Open a new ROM.\1OpenRom,', content)
content = re.sub(r'(\s+)OpenCheats,', r'\1/// Switch to the cheats panel.\1OpenCheats,', content)
content = re.sub(r'(\s+)SaveSlot\(u8\),', r'\1/// Save the state to a specific slot.\1SaveSlot(u8),', content)
content = re.sub(r'(\s+)LoadSlot\(u8\),', r'\1/// Load the state from a specific slot.\1LoadSlot(u8),', content)
content = re.sub(r'(\s+)Reset,', r'\1/// Reset the emulator.\1Reset,', content)
content = re.sub(r'(\s+)Quit,', r'\1/// Quit the application.\1Quit,', content)

# Add docs for CheatsSelection
content = re.sub(r'(\s+)AddCode,', r'\1/// Input a new cheat code.\1AddCode,', content)
content = re.sub(r'(\s+)Cheat\(usize\),', r'\1/// Toggle or remove an existing cheat code.\1Cheat(usize),', content)

# Add docs for OverlaySelection
content = re.sub(r'(\s+)Main\(MainMenuSelection\),', r'\1/// An item in the main menu.\1Main(MainMenuSelection),', content)
content = re.sub(r'(\s+)Cheats\(CheatsSelection\),', r'\1/// An item in the cheats menu.\1Cheats(CheatsSelection),', content)

# Add docs for OverlayCommand
content = re.sub(r'(\s+)AppAction\(AppAction\),', r'\1/// Execute a standard app action.\1AppAction(AppAction),', content)
content = re.sub(r'(\s+)ToggleCheat\(usize\),', r'\1/// Toggle a cheat code active state.\1ToggleCheat(usize),', content)
content = re.sub(r'(\s+)RemoveCheat\(usize\),', r'\1/// Remove a cheat code entirely.\1RemoveCheat(usize),', content)
content = re.sub(r'(\s+)SubmitCheatCode\(String\),', r'\1/// Submit a new cheat code string.\1SubmitCheatCode(String),', content)

# Add docs for OverlaySlotSummary
content = re.sub(r'(\s+)pub slot: u8,', r'\1/// The slot number.\1pub slot: u8,', content)
content = re.sub(r'(\s+)pub status_label: &\'static str,', r'\1/// Status label (e.g., "Empty" or "Saved").\1pub status_label: &\'static str,', content)
content = re.sub(r'(\s+)pub modified_unix_secs: Option<u64>,', r'\1/// Unix timestamp when the slot was last modified.\1pub modified_unix_secs: Option<u64>,', content)

# Add docs for OverlayCheatSummary
content = re.sub(r'(\s+)pub raw_code: &\'a str,', r'\1/// The raw string code.\1pub raw_code: &\'a str,', content)
content = re.sub(r'(\s+)pub enabled: bool,', r'\1/// Whether the cheat is currently active.\1pub enabled: bool,', content)


with open('crates/nes-desktop/src/overlay.rs', 'w') as f:
    f.write(content)
