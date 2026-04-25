import re

with open('crates/nes-test-harness/src/lib.rs', 'r') as f:
    content = f.read()

content = content.replace(
    'pub fn detect_mapper_id(rom_bytes: &[u8]) -> Option<u16> {',
    '/// Parses the iNES header of a ROM file to determine its memory mapper ID.\n/// Returns `None` if the header is missing or malformed.\n#[must_use]\npub fn detect_mapper_id(rom_bytes: &[u8]) -> Option<u16> {'
)
# Oops, already did that one, let's grep for the exact line to fix.
