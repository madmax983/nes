import re

content = open('crates/nes-desktop/src/input.rs').read()

print("Checking nesting inside classify_keyboard_input...")
m = re.search(r'pub\(crate\) fn classify_keyboard_input\((.*?)\) -> KeyboardDecision \{(.*?)\}', content, re.DOTALL)
if m:
    print(m.group(2))
