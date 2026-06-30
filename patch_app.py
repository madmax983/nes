import re

with open('crates/nes-desktop/src/app.rs', 'r') as f:
    content = f.read()

content = content.replace(
    'pub struct BridgeCommand {',
    '/// Represents a command sent from the application bridge to the emulator core.\npub struct BridgeCommand {'
)
content = re.sub(
    r'(\s+)pub core: Command,',
    r'\1/// The underlying core command.\1pub core: Command,',
    content
)
content = content.replace(
    'pub fn map_key_event_to_command(',
    '/// Maps a keyboard event code to a BridgeCommand.\npub fn map_key_event_to_command('
)
content = content.replace(
    'pub fn map_key_event_to_button(',
    '/// Maps a keyboard event code to a NES controller button.\npub fn map_key_event_to_button('
)
content = content.replace(
    'pub fn map_key_event_to_button_bit(',
    '/// Maps a keyboard event code to the bitmask of a NES controller button.\npub fn map_key_event_to_button_bit('
)

with open('crates/nes-desktop/src/app.rs', 'w') as f:
    f.write(content)
