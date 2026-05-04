import re

with open('crates/nes-mcp/src/dispatch.rs', 'r') as f:
    content = f.read()

# Add imports for the new parsing tests
new_imports = "parse_required_string, parse_dsl_source, parse_rom_payload, parse_u8, parse_u16,"
content = content.replace("parse_dsl_rom_options, parse_hex_bytes, parse_player2, parse_slot, parse_speed_permille,", f"parse_dsl_rom_options, parse_hex_bytes, parse_player2, parse_slot, parse_speed_permille, {new_imports}")

with open('crates/nes-mcp/src/dispatch.rs', 'w') as f:
    f.write(content)
