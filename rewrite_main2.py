with open("crates/nes-desktop/src/main.rs", "r") as f:
    data = f.read()

import re

# Remove connected_gamepad_ids and gamepad_snapshot_to_bits
funcs = ["connected_gamepad_ids", "gamepad_snapshot_to_bits"]
for func in funcs:
    pattern = r'fn ' + func + r'\s*\([^)]*\)(?:\s*->\s*[^{]+)?\s*\{.*?\n\}\n'
    data = re.sub(pattern, '', data, flags=re.DOTALL)

with open("crates/nes-desktop/src/main.rs", "w") as f:
    f.write(data)
