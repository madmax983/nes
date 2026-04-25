import re

with open('crates/nes-ai/src/episode.rs', 'r') as f:
    content = f.read()

content = content.replace(
    '    #[must_use]\n    pub fn new(output_dir: PathBuf) -> Self {',
    '    /// Binds the writer to a specific output directory.\n    #[must_use]\n    pub fn new(output_dir: PathBuf) -> Self {'
)

with open('crates/nes-ai/src/episode.rs', 'w') as f:
    f.write(content)
