import re

with open('crates/nes-ai/src/env.rs', 'r') as f:
    content = f.read()

content = content.replace(
    'pub enum AnyControlEnv {',
    '/// Exposes the specific environment implementations supported by the AI pipeline.\npub enum AnyControlEnv {'
)
content = content.replace(
    '    Smb(ProfileEnv<SmbProfile>),',
    '    /// Environment tuned for Super Mario Bros using the `SmbProfile`.\n    Smb(ProfileEnv<SmbProfile>),'
)

content = content.replace(
    '    #[must_use]\n    pub fn recorded_movie(&self) -> &TasMovie {',
    '    /// Retrieves a reference to the active TAS recording.\n    #[must_use]\n    pub fn recorded_movie(&self) -> &TasMovie {'
)


with open('crates/nes-ai/src/env.rs', 'w') as f:
    f.write(content)

with open('crates/nes-ai/src/episode.rs', 'r') as f:
    content = f.read()

content = content.replace(
    '#[derive(Debug, Clone, Serialize)]\npub struct EpisodeMetadata {',
    '/// Standard metadata collected over the lifetime of a single training episode.\n#[derive(Debug, Clone, Serialize)]\npub struct EpisodeMetadata {'
)
content = content.replace(
    '    pub profile_id: String,\n    pub snapshot_id: String,\n    pub rom_hash: String,\n    pub total_reward: f32,\n    pub episode_frames: u64,\n    pub final_state_hash: u64,',
    '''    /// The profile this episode was evaluated under.
    pub profile_id: String,
    /// The unique identifier of the starting state snapshot.
    pub snapshot_id: String,
    /// The SHA-256 hash of the ROM.
    pub rom_hash: String,
    /// The total accumulated reward (return).
    pub total_reward: f32,
    /// Total number of emulator frames elapsed.
    pub episode_frames: u64,
    /// A structural hash of the emulator\'s final state.
    pub final_state_hash: u64,'''
)

content = content.replace(
    '#[derive(Debug, Clone)]\npub struct EpisodeArtifactPaths {',
    '/// Contains the paths to the artifacts generated during an evaluation episode.\n#[derive(Debug, Clone)]\npub struct EpisodeArtifactPaths {'
)
content = content.replace(
    '    pub tas_json_path: PathBuf,\n    pub run_json_path: PathBuf,\n    pub macro_txt_path: Option<PathBuf>,',
    '''    /// The path to the saved TAS input replay movie.
    pub tas_json_path: PathBuf,
    /// The path to the saved episode metadata JSON file.
    pub run_json_path: PathBuf,
    /// The path to the saved macro text file (if exported).
    pub macro_txt_path: Option<PathBuf>,'''
)

content = content.replace(
    '#[derive(Debug, Clone)]\npub struct EpisodeArtifactWriter {',
    '/// A utility for writing episode recording artifacts to disk.\n#[derive(Debug, Clone)]\npub struct EpisodeArtifactWriter {'
)

content = content.replace(
    '    pub fn new(output_dir: PathBuf) -> Result<Self, AiError> {',
    '    /// Creates a new `EpisodeArtifactWriter` ensuring the artifact directory exists.\n    pub fn new(output_dir: PathBuf) -> Result<Self, AiError> {'
)

with open('crates/nes-ai/src/episode.rs', 'w') as f:
    f.write(content)
