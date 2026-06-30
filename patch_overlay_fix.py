import re

with open('crates/nes-desktop/src/overlay.rs', 'r') as f:
    content = f.read()

content = content.replace(r"pub status_label: &\'static str,", "pub status_label: &'static str,")
content = content.replace(r"pub raw_code: &\'a str,", "pub raw_code: &'a str,")

with open('crates/nes-desktop/src/overlay.rs', 'w') as f:
    f.write(content)
