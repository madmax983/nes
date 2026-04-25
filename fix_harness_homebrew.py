import re

with open('crates/nes-test-harness/src/homebrew.rs', 'r') as f:
    content = f.read()

content = content.replace(
    '#[must_use]\npub fn default_homebrew_rom_path() -> PathBuf {',
    '/// Returns the absolute path where the default homebrew ROM should be located within the repository.\n#[must_use]\npub fn default_homebrew_rom_path() -> PathBuf {'
)

with open('crates/nes-test-harness/src/homebrew.rs', 'w') as f:
    f.write(content)
