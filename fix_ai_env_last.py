import re

with open('crates/nes-ai/src/env.rs', 'r') as f:
    content = f.read()

content = content.replace(
    'pub enum AnyControlAction {',
    '/// Actions compatible with `AnyControlEnv`.\npub enum AnyControlAction {'
)
content = content.replace(
    '    Discrete(ControlAction),',
    '    /// A standard controller action.\n    Discrete(ControlAction),'
)
content = content.replace(
    'pub type AnyControlEnv = ProfileEnv<Box<dyn TaskProfile>>;',
    '/// A type-erased wrapper for any control-based environment profile.\npub type AnyControlEnv = ProfileEnv<Box<dyn TaskProfile>>;'
)

with open('crates/nes-ai/src/env.rs', 'w') as f:
    f.write(content)

with open('crates/nes-ai/src/episode.rs', 'r') as f:
    content = f.read()

content = content.replace(
    '    pub fn write_metadata(',
    '    /// Writes the episode metadata to a formatted JSON file.\n    pub fn write_metadata('
)

with open('crates/nes-ai/src/episode.rs', 'w') as f:
    f.write(content)
