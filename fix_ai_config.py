import re

with open('crates/nes-ai/src/config.rs', 'r') as f:
    content = f.read()

content = content.replace(
    'pub const MIN_OBSERVATION_DIM: usize = 20;',
    '/// Minimum valid dimension for observation pixel tensors.\n/// Prevent configurations from scaling inputs down so far that the network cannot identify objects.\npub const MIN_OBSERVATION_DIM: usize = 20;'
)

content = content.replace(
    '#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize)]\n#[serde(rename_all = "kebab-case")]\npub enum GameProfileId {',
    '/// Identifies a specific NES game targeted for reinforcement learning.\n#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize)]\n#[serde(rename_all = "kebab-case")]\npub enum GameProfileId {'
)
content = content.replace(
    '    #[default]\n    Smb,',
    '    /// Super Mario Bros (NTSC version).\n    #[default]\n    Smb,'
)

content = content.replace(
    '#[derive(Debug, Clone, Deserialize)]\n#[serde(deny_unknown_fields)]\npub struct AiProfileConfig {',
    '/// Core parameters governing how the environment is configured and stepped during training.\n#[derive(Debug, Clone, Deserialize)]\n#[serde(deny_unknown_fields)]\npub struct AiProfileConfig {'
)

content = content.replace(
    '    #[serde(default)]\n    pub game: GameProfileId,\n    pub id: String,\n    pub rom_path: PathBuf,\n    pub snapshot_path: PathBuf,\n    pub bootstrap_tas_path: PathBuf,\n    #[serde(deserialize_with = "deserialize_positive_usize")]\n    pub frame_stack: usize,\n    #[serde(deserialize_with = "deserialize_positive_u32")]\n    pub frame_skip: u32,\n    #[serde(deserialize_with = "deserialize_positive_u32")]\n    pub max_episode_frames: u32,\n    pub observation: ObservationConfig,\n    pub reward: RewardConfig,',
    '''    /// The target game for this profile.
    #[serde(default)]
    pub game: GameProfileId,
    /// A unique string identifier for this profile configuration.
    pub id: String,
    /// Path to the ROM file required to boot the environment.
    pub rom_path: PathBuf,
    /// Path to a JSON snapshot used to inject the initial memory state.
    pub snapshot_path: PathBuf,
    /// Path to a TAS recording used to step the environment to a specific start state.
    pub bootstrap_tas_path: PathBuf,
    /// Number of consecutive observations to stack together as input.
    #[serde(deserialize_with = "deserialize_positive_usize")]
    pub frame_stack: usize,
    /// Number of emulator frames to advance per action step.
    #[serde(deserialize_with = "deserialize_positive_u32")]
    pub frame_skip: u32,
    /// Hard limit on the number of frames allowed before truncating an episode.
    #[serde(deserialize_with = "deserialize_positive_u32")]
    pub max_episode_frames: u32,
    /// Configuration for downscaling the visual observation tensor.
    pub observation: ObservationConfig,
    /// Weights and parameters governing the reward signal.
    pub reward: RewardConfig,'''
)


content = content.replace(
    '#[derive(Debug, Clone, Deserialize)]\n#[serde(deny_unknown_fields)]\npub struct ObservationConfig {',
    '/// Defines how the raw 256x240 NES framebuffer is processed into a model input.\n#[derive(Debug, Clone, Deserialize)]\n#[serde(deny_unknown_fields)]\npub struct ObservationConfig {'
)
content = content.replace(
    '    #[serde(deserialize_with = "deserialize_observation_dim")]\n    pub width: usize,\n    #[serde(deserialize_with = "deserialize_observation_dim")]\n    pub height: usize,',
    '''    /// The target width in pixels.
    #[serde(deserialize_with = "deserialize_observation_dim")]
    pub width: usize,
    /// The target height in pixels.
    #[serde(deserialize_with = "deserialize_observation_dim")]
    pub height: usize,'''
)

content = content.replace(
    '#[derive(Debug, Clone, Copy, Deserialize)]\n#[serde(deny_unknown_fields)]\npub struct RewardConfig {',
    '/// Contains coefficients and thresholds used to shape the reinforcement learning reward.\n#[derive(Debug, Clone, Copy, Deserialize)]\n#[serde(deny_unknown_fields)]\npub struct RewardConfig {'
)

content = content.replace(
    '    pub forward_progress: f32,\n    pub alive_bonus: f32,\n    pub stall_penalty: f32,\n    pub death_penalty: f32,\n    #[serde(deserialize_with = "deserialize_positive_u32")]\n    pub stall_frames: u32,',
    '''    /// Scalar reward applied per unit of progress.
    pub forward_progress: f32,
    /// Flat scalar reward applied on every frame the agent survives.
    pub alive_bonus: f32,
    /// Flat scalar penalty applied when the agent triggers the stall condition.
    pub stall_penalty: f32,
    /// Flat scalar penalty applied when the agent dies.
    pub death_penalty: f32,
    /// Consecutive frames with no progress required to trigger the stall penalty.
    #[serde(deserialize_with = "deserialize_positive_u32")]
    pub stall_frames: u32,'''
)


with open('crates/nes-ai/src/config.rs', 'w') as f:
    f.write(content)
