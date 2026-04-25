import re

with open('crates/nes-test-harness/src/lib.rs', 'r') as f:
    content = f.read()

content = content.replace(
    '#[must_use]\npub fn mapper_supported_by_core(mapper_id: u16) -> bool {',
    '/// Returns true if the core supports the given mapper ID.\n#[must_use]\npub fn mapper_supported_by_core(mapper_id: u16) -> bool {'
)

with open('crates/nes-test-harness/src/lib.rs', 'w') as f:
    f.write(content)
