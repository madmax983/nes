import re

with open('crates/nes-ai/src/env.rs', 'r') as f:
    content = f.read()

content = content.replace(
    '    Smb(Box<SmbControlEnv>),',
    '    /// Environment tuned for Super Mario Bros using the `SmbProfile`.\n    Smb(Box<SmbControlEnv>),'
)

with open('crates/nes-ai/src/env.rs', 'w') as f:
    f.write(content)

with open('crates/nes-ai/src/episode.rs', 'r') as f:
    content = f.read()

content = content.replace(
    '    pub fn base_path(&self) -> &std::path::Path {',
    '    /// Returns the underlying base directory where artifacts are written.\n    pub fn base_path(&self) -> &std::path::Path {'
)

with open('crates/nes-ai/src/episode.rs', 'w') as f:
    f.write(content)
