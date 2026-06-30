import re

with open('crates/nes-desktop/src/manual_state.rs', 'r') as f:
    content = f.read()

# Add docs for SaveSlotStatus
content = content.replace(
    'pub enum SaveSlotStatus {',
    '/// Status of a save slot file on disk.\npub enum SaveSlotStatus {'
)
content = re.sub(
    r'(\s+)Empty,',
    r'\1/// No save file exists in this slot.\1Empty,',
    content
)
content = re.sub(
    r'(\s+)Saved,',
    r'\1/// A valid save file exists.\1Saved,',
    content
)
content = re.sub(
    r'(\s+)Corrupt,',
    r'\1/// The save file is corrupted or unreadable.\1Corrupt,',
    content
)
content = re.sub(
    r'(\s+)IncompatibleRom,',
    r'\1/// The save file belongs to a different ROM hash.\1IncompatibleRom,',
    content
)

# Add docs for SaveSlotMetadata
content = content.replace(
    'pub struct SaveSlotMetadata {',
    '/// Metadata describing the state of a save slot.\npub struct SaveSlotMetadata {'
)
content = re.sub(
    r'(\s+)pub slot: u8,',
    r'\1/// Slot index (1-based).\1pub slot: u8,',
    content
)
content = re.sub(
    r'(\s+)pub path: PathBuf,',
    r'\1/// Disk path to the save file.\1pub path: PathBuf,',
    content
)
content = re.sub(
    r'(\s+)pub status: SaveSlotStatus,',
    r'\1/// The integrity status of the slot.\1pub status: SaveSlotStatus,',
    content
)
content = re.sub(
    r'(\s+)pub modified_unix_secs: Option<u64>,',
    r'\1/// Unix timestamp when the file was last modified.\1pub modified_unix_secs: Option<u64>,',
    content
)

# Add docs for functions
content = content.replace(
    'pub fn resolve_save_slots(',
    '/// Scans the save directory and resolves the status of multiple save slots for a ROM.\npub fn resolve_save_slots('
)
content = content.replace(
    'pub fn write_save_state(',
    '/// Serializes and writes a `CoreSnapshot` to a specific save slot.\npub fn write_save_state('
)
content = content.replace(
    'pub fn read_save_state(',
    '/// Reads and deserializes a `CoreSnapshot` from a specific save slot.\npub fn read_save_state('
)


with open('crates/nes-desktop/src/manual_state.rs', 'w') as f:
    f.write(content)
