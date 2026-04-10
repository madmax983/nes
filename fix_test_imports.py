with open('crates/nes-desktop/src/main.rs', 'r') as f:
    content = f.read()

content = content.replace("format_rom_read_error, ", "")
content = content.replace("format_rom_read_error,\n", "")

with open('crates/nes-desktop/src/main.rs', 'w') as f:
    f.write(content)
