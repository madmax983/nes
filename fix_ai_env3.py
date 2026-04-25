import re

with open('crates/nes-ai/src/env.rs', 'r') as f:
    content = f.read()

content = content.replace(
    '    #[must_use]\n    pub fn new(profile: P, snapshot: SnapshotBundle) -> Self {',
    '    /// Internal constructor used after generic parameter resolution.\n    #[must_use]\n    pub fn new(profile: P, snapshot: SnapshotBundle) -> Self {'
)

content = content.replace(
    '    #[must_use]\n    pub fn recorded_movie(&self) -> &nes_core::tas::TasMovie {',
    '    /// Exposes the TAS recording constructed during the current episode.\n    #[must_use]\n    pub fn recorded_movie(&self) -> &nes_core::tas::TasMovie {'
)

content = content.replace(
    '    #[must_use]\n    pub fn finish_episode(&self, total_reward: f32) -> EpisodeMetadata {',
    '    /// Constructs the final episode metadata struct by aggregating internal tracking stats.\n    #[must_use]\n    pub fn finish_episode(&self, total_reward: f32) -> EpisodeMetadata {'
)

with open('crates/nes-ai/src/env.rs', 'w') as f:
    f.write(content)
