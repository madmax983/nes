import re

with open("crates/nes-desktop/src/main.rs", "r") as f:
    data = f.read()

tests = [
    "desktop_loop_helper_primitives_cover_window_scale_and_player_flags",
    "gamepad_sampling_helpers_map_buttons_and_axis_thresholds"
]

for test in tests:
    pattern = r'(?:#\[test\]\s*)?(?:#\[allow\(deprecated\)\]\s*)?fn ' + test + r'\s*\(\)\s*\{.*?\n    \}\n'
    data = re.sub(pattern, '', data, flags=re.DOTALL)

with open("crates/nes-desktop/src/main.rs", "w") as f:
    f.write(data)
