import re

with open('crates/nes-ai/src/env.rs', 'r') as f:
    content = f.read()

content = content.replace(
    '    pub fn new(\n        config: &AiProfileConfig,\n        rom_bytes: &[u8],\n    ) -> Result<Self, AiError> {',
    '    /// Initializes the environment from an `AiProfileConfig` and raw ROM bytes.\n    ///\n    /// Instantiates the NES core, verifies the ROM hash, loads the boot snapshot,\n    /// and executes the initialization TAS sequence.\n    pub fn new(\n        config: &AiProfileConfig,\n        rom_bytes: &[u8],\n    ) -> Result<Self, AiError> {'
)

# Since we had a few method warnings:
content = content.replace(
    '    pub fn step(&mut self, action: AnyControlAction) -> StepOutput<ObservationSnapshot> {',
    '    /// Dynamically applies an action to the active profile environment.\n    pub fn step(&mut self, action: AnyControlAction) -> StepOutput<ObservationSnapshot> {'
)
content = content.replace(
    '    pub fn reset_to_snapshot(&mut self) -> Result<ObservationSnapshot, AiError> {',
    '    /// Resets the environment to the beginning of the episode using the profile\'s snapshot.\n    ///\n    /// Flushes any pending rewards, clears the frame stack, loads the saved state,\n    /// replays the initialization TAS, and captures the fresh observation.\n    pub fn reset_to_snapshot(&mut self) -> Result<ObservationSnapshot, AiError> {'
)

# Replace the trait-like ones on AnyControlEnv:
content = content.replace(
    'impl AnyControlEnv {\n    pub fn from_config(',
    'impl AnyControlEnv {\n    /// Dispatches configuration logic dynamically based on the game identifier.\n    ///\n    /// Returns an instantiated `AnyControlEnv` equipped with the appropriate internal profile.\n    pub fn from_config('
)

content = content.replace(
    '    pub fn step(&mut self, action: AnyControlAction) -> StepOutput<ObservationSnapshot> {',
    '    /// Dynamically applies an action to the active profile environment.\n    pub fn step(&mut self, action: AnyControlAction) -> StepOutput<ObservationSnapshot> {'
)
content = content.replace(
    '    pub fn reset_to_snapshot(&mut self) -> Result<ObservationSnapshot, AiError> {',
    '    /// Dynamically resets the active profile environment to its snapshot.\n    pub fn reset_to_snapshot(&mut self) -> Result<ObservationSnapshot, AiError> {'
)

with open('crates/nes-ai/src/env.rs', 'w') as f:
    f.write(content)
