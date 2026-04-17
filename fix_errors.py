import re

with open("crates/nes-ai/src/error.rs", "r") as f:
    content = f.read()

content = content.replace('{source}\\n{} Ensure the file exists or check the profile configuration."', '{source}\\nHint: Ensure the file exists or check the profile configuration."')
content = content.replace('{source}\\n{} Ensure the ROM file exists and is accessible."', '{source}\\nHint: Ensure the ROM file exists and is accessible."')

with open("crates/nes-ai/src/error.rs", "w") as f:
    f.write(content)
