import re

with open('crates/nes-desktop/src/manual_state.rs', 'r') as f:
    content = f.read()

content = content.replace(
    'pub fn slot_path_for_rom(',
    '/// Generates the deterministic path for a specific ROM save slot.\npub fn slot_path_for_rom('
)
content = content.replace(
    'pub fn slot_paths_for_rom(',
    '/// Generates paths for a range of save slots associated with a ROM.\npub fn slot_paths_for_rom('
)
content = content.replace(
    'pub fn quicksave_path_for_rom(',
    '/// Generates the path for the legacy unnumbered quicksave slot.\npub fn quicksave_path_for_rom('
)

with open('crates/nes-desktop/src/manual_state.rs', 'w') as f:
    f.write(content)
