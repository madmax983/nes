import re

with open('crates/nes-ai/src/env.rs', 'r') as f:
    content = f.read()

content = content.replace(
    'pub type SmbControlEnv = ProfileEnv<SmbProfile>;',
    '/// Standard alias for the `ProfileEnv` specialized to Super Mario Bros.\npub type SmbControlEnv = ProfileEnv<SmbProfile>;'
)

with open('crates/nes-ai/src/env.rs', 'w') as f:
    f.write(content)

with open('crates/nes-ai/src/episode.rs', 'r') as f:
    content = f.read()

content = content.replace(
    '    pub fn write_macro_script(',
    '    /// Serializes the macro script representation and writes it to disk.\n    pub fn write_macro_script('
)

with open('crates/nes-ai/src/episode.rs', 'w') as f:
    f.write(content)
