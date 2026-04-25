import re

with open('crates/nes-ai/src/env.rs', 'r') as f:
    content = f.read()

content = content.replace(
    '#[derive(Debug, Clone)]\npub struct StepOutput<F> {',
    '/// Data returned when the environment is advanced by one action step.\n#[derive(Debug, Clone)]\npub struct StepOutput<F> {'
)
content = content.replace(
    '    pub features: F,\n    pub reward: RewardBreakdown,\n    pub done: bool,',
    '''    /// Observations or features produced by the step.
    pub features: F,
    /// Detailed breakdown of the reward components.
    pub reward: RewardBreakdown,
    /// True if the episode has reached a terminal state (e.g. death or win).
    pub done: bool,'''
)

content = content.replace(
    '#[derive(Debug, Clone, PartialEq)]\npub struct ObservationSnapshot {',
    '/// A bundled snapshot of the visual inputs provided to the neural network.\n#[derive(Debug, Clone, PartialEq)]\npub struct ObservationSnapshot {'
)
content = content.replace(
    '    pub frame_stack: usize,\n    pub width: usize,\n    pub height: usize,\n    pub frames: Vec<f32>,\n    pub features: Vec<f32>,',
    '''    /// Number of sequential frames in this snapshot.
    pub frame_stack: usize,
    /// Width of each frame in pixels.
    pub width: usize,
    /// Height of each frame in pixels.
    pub height: usize,
    /// Flattened 1D array of pixel data across all frames.
    pub frames: Vec<f32>,
    /// Any additional numerical features extracted from memory.
    pub features: Vec<f32>,'''
)

content = content.replace(
    '#[derive(Debug, Clone)]\npub struct ControlStepOutput {',
    '/// Output structure used internally during control actions before observation processing.\n#[derive(Debug, Clone)]\npub struct ControlStepOutput {'
)
content = content.replace(
    '    pub reward: RewardBreakdown,\n    pub done: bool,',
    '''    /// Broken down components of the reward function.
    pub reward: RewardBreakdown,
    /// Whether the terminal conditions of the environment were met.
    pub done: bool,'''
)


content = content.replace(
    '    pub fn new(\n        config: &AiProfileConfig,\n        rom_bytes: &[u8],\n    ) -> Result<Self, AiError> {',
    '    /// Initializes the environment from an `AiProfileConfig` and raw ROM bytes.\n    ///\n    /// Instantiates the NES core, verifies the ROM hash, loads the boot snapshot,\n    /// and executes the initialization TAS sequence.\n    pub fn new(\n        config: &AiProfileConfig,\n        rom_bytes: &[u8],\n    ) -> Result<Self, AiError> {'
)

content = content.replace(
    '    pub fn core(&self) -> &NesCore {',
    '    /// Returns a shared reference to the underlying `NesCore`.\n    pub fn core(&self) -> &NesCore {'
)

content = content.replace(
    '    pub fn core_mut(&mut self) -> &mut NesCore {',
    '    /// Returns a mutable reference to the underlying `NesCore`.\n    pub fn core_mut(&mut self) -> &mut NesCore {'
)

content = content.replace(
    '    pub fn step(&mut self, action: ControlAction) -> StepOutput<ObservationSnapshot> {',
    '    /// Steps the environment using a given action.\n    ///\n    /// Takes the specified number of `frame_skip` emulator steps, aggregates rewards,\n    /// computes the new downscaled observation stack, and evaluates terminal conditions.\n    pub fn step(&mut self, action: ControlAction) -> StepOutput<ObservationSnapshot> {'
)

content = content.replace(
    '    pub fn reset_to_snapshot(&mut self) -> Result<ObservationSnapshot, AiError> {',
    '    /// Resets the environment to the beginning of the episode using the profile\'s snapshot.\n    ///\n    /// Flushes any pending rewards, clears the frame stack, loads the saved state,\n    /// replays the initialization TAS, and captures the fresh observation.\n    pub fn reset_to_snapshot(&mut self) -> Result<ObservationSnapshot, AiError> {'
)


content = content.replace(
    'pub type AnyControlEnv = ProfileEnv<Box<dyn TaskProfile>>;\n\n#[derive(Debug, Clone, Copy, PartialEq, Eq)]\npub enum AnyControlAction {\n    Discrete(ControlAction),\n}',
    '/// A type-erased wrapper for any control-based environment profile.\npub type AnyControlEnv = ProfileEnv<Box<dyn TaskProfile>>;\n\n/// Actions compatible with `AnyControlEnv`.\n#[derive(Debug, Clone, Copy, PartialEq, Eq)]\npub enum AnyControlAction {\n    /// A standard controller action.\n    Discrete(ControlAction),\n}'
)


content = content.replace(
    '    pub fn from_config(\n        config: AiProfileConfig,\n        rom_bytes: &[u8],\n    ) -> Result<Self, AiError> {',
    '    /// Dispatches configuration logic dynamically based on the game identifier.\n    ///\n    /// Returns an instantiated `AnyControlEnv` equipped with the appropriate internal profile.\n    pub fn from_config(\n        config: AiProfileConfig,\n        rom_bytes: &[u8],\n    ) -> Result<Self, AiError> {'
)

content = content.replace(
    '    pub fn step(&mut self, action: AnyControlAction) -> StepOutput<ObservationSnapshot> {',
    '    /// Dynamically applies an action to the active profile environment.\n    pub fn step(&mut self, action: AnyControlAction) -> StepOutput<ObservationSnapshot> {'
)


with open('crates/nes-ai/src/env.rs', 'w') as f:
    f.write(content)
