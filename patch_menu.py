import re

with open('crates/nes-desktop/src/menu.rs', 'r') as f:
    content = f.read()

content = content.replace(
    'pub fn entries(&self) -> &[DesktopMenuEntry] {',
    '/// Returns a slice of the logical menu tree.\n    pub fn entries(&self) -> &[DesktopMenuEntry] {'
)
content = content.replace(
    'pub fn install_for_window(&self, window: &Window) -> Result<(), String> {',
    '/// Attaches the native menu to the main window.\n    pub fn install_for_window(&self, window: &Window) -> Result<(), String> {'
)
content = content.replace(
    'pub fn poll_action(&self) -> Option<AppAction> {',
    '/// Checks for any pending menu events fired by the native platform.\n    pub fn poll_action(&self) -> Option<AppAction> {'
)
content = content.replace(
    'pub fn set_action_enabled(&self, action: AppAction, enabled: bool) {',
    '/// Toggles the interactive state of a specific menu item.\n    pub fn set_action_enabled(&self, action: AppAction, enabled: bool) {'
)
content = content.replace(
    'pub fn sync_runtime_state(&self, rollback_enabled: bool) {',
    '/// Synchronizes the disabled/enabled states of the menu items based on runtime conditions.\n    pub fn sync_runtime_state(&self, rollback_enabled: bool) {'
)
content = re.sub(
    r'(\s+)Item\(MenuItemSpec\),',
    r'\1/// A standard actionable item.\1Item(MenuItemSpec),',
    content
)
content = re.sub(
    r'(\s+)Separator,',
    r'\1/// A visual divider line.\1Separator,',
    content
)
content = re.sub(
    r'(\s+)Submenu \{',
    r'\1/// A nested group of menu entries.\1Submenu {',
    content
)
content = re.sub(
    r'(\s+)label: &\'static str,',
    r'\1/// The display text for the submenu.\1label: &\'static str,',
    content
)
content = re.sub(
    r'(\s+)entries: Vec<DesktopMenuEntry>,',
    r'\1/// The items inside the submenu.\1entries: Vec<DesktopMenuEntry>,',
    content
)
content = re.sub(
    r'(\s+)pub id: String,',
    r'\1/// The unique routing ID for this action.\1pub id: String,',
    content
)
content = re.sub(
    r'(\s+)pub label: String,',
    r'\1/// The display text for the item.\1pub label: String,',
    content
)
content = content.replace(
    'pub fn build_desktop_menu(has_rom: bool) -> DesktopMenu {',
    '/// Constructs the full application menu hierarchy.\npub fn build_desktop_menu(has_rom: bool) -> DesktopMenu {'
)
content = content.replace(
    'pub fn open_rom_dialog(start_dir: Option<PathBuf>) -> Option<PathBuf> {',
    '/// Blocks the thread to display a native file-picker dialog for selecting a ROM.\npub fn open_rom_dialog(start_dir: Option<PathBuf>) -> Option<PathBuf> {'
)


with open('crates/nes-desktop/src/menu.rs', 'w') as f:
    f.write(content)
