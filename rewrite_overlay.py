import re

with open("crates/nes-desktop/src/overlay.rs", "r") as f:
    code = f.read()

print("Original contains render_suffix() -> String:", "pub fn render_suffix(&self) -> String" in code)
