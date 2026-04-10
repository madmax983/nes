with open('crates/nes-desktop/src/main.rs', 'r') as f:
    content = f.read()

content = content.replace("DEFAULT_CAPTURE_EVERY_FRAMES, ", "")
content = content.replace("capture_config_from_parts, ", "")
content = content.replace("capture_path_for_frame,\n", "\n")
content = content.replace("recommended_input_delay_frames,\n", "\n")

with open('crates/nes-desktop/src/main.rs', 'w') as f:
    f.write(content)
