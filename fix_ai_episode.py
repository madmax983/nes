import re

with open('crates/nes-ai/src/episode.rs', 'r') as f:
    content = f.read()

content = content.replace(
    '#[derive(Debug, Clone)]\npub struct EpisodeMetadata {',
    '/// Standard metadata collected over the lifetime of a single training episode.\n#[derive(Debug, Clone)]\npub struct EpisodeMetadata {'
)
content = content.replace(
    '    pub step_count: u32,\n    pub frame_count: u32,\n    pub return_sum: f32,\n    pub death_count: u32,\n    pub stall_count: u32,\n    pub reward_features: RewardFeatures,',
    '''    /// Number of environment steps taken.
    pub step_count: u32,
    /// Total number of emulator frames elapsed.
    pub frame_count: u32,
    /// The total accumulated reward.
    pub return_sum: f32,
    /// Number of times the agent died during the episode.
    pub death_count: u32,
    /// Number of times the agent stalled (made no progress) and was penalized.
    pub stall_count: u32,
    /// The final values of the tracked reward features.
    pub reward_features: RewardFeatures,'''
)

content = content.replace(
    '#[derive(Debug, Clone)]\npub struct EpisodeArtifacts {',
    '/// Contains the paths to the artifacts generated during an evaluation episode.\n#[derive(Debug, Clone)]\npub struct EpisodeArtifacts {'
)
content = content.replace(
    '    pub metadata_json_path: PathBuf,\n    pub replay_tas_path: PathBuf,\n    pub initial_snapshot_path: PathBuf,',
    '''    /// The path to the saved episode metadata JSON file.
    pub metadata_json_path: PathBuf,
    /// The path to the saved TAS input replay movie.
    pub replay_tas_path: PathBuf,
    /// The path to the snapshot representing the start of the episode.
    pub initial_snapshot_path: PathBuf,'''
)

content = content.replace(
    'pub struct EpisodeWriter {',
    '/// A utility for writing episode recording artifacts to disk.\npub struct EpisodeWriter {'
)

content = content.replace(
    '    pub fn new(\n        run_id: String,\n        artifact_dir: PathBuf,\n    ) -> Result<Self, AiError> {',
    '    /// Creates a new `EpisodeWriter` ensuring the artifact directory exists.\n    pub fn new(\n        run_id: String,\n        artifact_dir: PathBuf,\n    ) -> Result<Self, AiError> {'
)

with open('crates/nes-ai/src/episode.rs', 'w') as f:
    f.write(content)
