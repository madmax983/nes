import re

with open('crates/nes-desktop/src/app.rs', 'r') as f:
    content = f.read()

content = content.replace(
    'pub fn tool_name(self) -> &\'static str {',
    '/// Returns a static string representing the tool name associated with this command.\n    pub fn tool_name(self) -> &\'static str {'
)

with open('crates/nes-desktop/src/app.rs', 'w') as f:
    f.write(content)
