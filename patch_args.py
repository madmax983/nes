import re

with open('crates/nes-desktop/src/args.rs', 'r') as f:
    content = f.read()

# Add docs for constants
content = content.replace(
    'pub const DEFAULT_MCP_BIND_ADDR: &str = "127.0.0.1:6502";',
    '/// The default network address for the MCP host to bind to.\npub const DEFAULT_MCP_BIND_ADDR: &str = "127.0.0.1:6502";'
)
content = content.replace(
    'pub const RUNTIME_USAGE: &str =',
    '/// Usage instructions printed when `--help` is requested or on parsing error.\npub const RUNTIME_USAGE: &str ='
)

# Add docs for fields in RuntimeArgs
content = re.sub(
    r'(\s+)pub rom_path: Option<String>,',
    r'\1/// Path to the NES ROM to execute.\1pub rom_path: Option<String>,',
    content
)
content = re.sub(
    r'(\s+)pub cheat_codes: Vec<String>,',
    r'\1/// A list of Game Genie or raw cheat codes to apply.\1pub cheat_codes: Vec<String>,',
    content
)
content = re.sub(
    r'(\s+)pub mcp_enabled: bool,',
    r'\1/// Whether to run an MCP server alongside the emulator.\1pub mcp_enabled: bool,',
    content
)
content = re.sub(
    r'(\s+)pub mcp_bind_addr: String,',
    r'\1/// Address for the MCP host to bind to.\1pub mcp_bind_addr: String,',
    content
)
content = re.sub(
    r'(\s+)pub netplay_enabled: bool,',
    r'\1/// Enable multiplayer Netplay over network.\1pub netplay_enabled: bool,',
    content
)
content = re.sub(
    r'(\s+)pub netplay_relay_addr: Option<String>,',
    r'\1/// Host address for the Netplay relay server.\1pub netplay_relay_addr: Option<String>,',
    content
)
content = re.sub(
    r'(\s+)pub netplay_room: Option<String>,',
    r'\1/// Name of the Netplay room to join.\1pub netplay_room: Option<String>,',
    content
)
content = re.sub(
    r'(\s+)pub netplay_player: Option<u8>,',
    r'\1/// Local player slot (1 or 2).\1pub netplay_player: Option<u8>,',
    content
)
content = re.sub(
    r'(\s+)pub netplay_input_delay_frames: Option<u32>,',
    r'\1/// Netplay rollback input delay override.\1pub netplay_input_delay_frames: Option<u32>,',
    content
)
content = re.sub(
    r'(\s+)pub netplay_max_rollback_frames: Option<u32>,',
    r'\1/// Maximum allowable rollback frames before disconnecting.\1pub netplay_max_rollback_frames: Option<u32>,',
    content
)
content = re.sub(
    r'(\s+)pub netplay_hash_check_every_frames: Option<u64>,',
    r'\1/// Interval for netplay state verification.\1pub netplay_hash_check_every_frames: Option<u64>,',
    content
)
content = re.sub(
    r'(\s+)pub rta_enabled: bool,',
    r'\1/// Enable strict RTA (Real-Time Attack) speedrun validation rules.\1pub rta_enabled: bool,',
    content
)
content = re.sub(
    r'(\s+)pub rta_profile_id: Option<String>,',
    r'\1/// Manual override for the RTA profile ID.\1pub rta_profile_id: Option<String>,',
    content
)
content = re.sub(
    r'(\s+)pub rta_profiles_dir: Option<String>,',
    r'\1/// Custom path to the RTA profiles directory.\1pub rta_profiles_dir: Option<String>,',
    content
)
content = re.sub(
    r'(\s+)pub rta_runs_dir: Option<String>,',
    r'\1/// Custom path to the RTA runs artifact directory.\1pub rta_runs_dir: Option<String>,',
    content
)
content = re.sub(
    r'(\s+)pub rta_calibrate: bool,',
    r'\1/// Enable RTA calibration draft generation mode.\1pub rta_calibrate: bool,',
    content
)
content = re.sub(
    r'(\s+)#\[cfg\(feature = "nova"\)\]\s+pub auto_player_enabled: bool,',
    r'\1#[cfg(feature = "nova")]\1/// Enable autonomous gameplay.\1pub auto_player_enabled: bool,',
    content
)


with open('crates/nes-desktop/src/args.rs', 'w') as f:
    f.write(content)
