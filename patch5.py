import re

with open('crates/nes-desktop/src/rta.rs', 'r') as f:
    content = f.read()

# Add docs for serde_iter
content = content.replace(
    'pub mod serde_iter {',
    '/// Helper module for serializing iterators with `serde`.\npub mod serde_iter {'
)

# Add docs for DraftOutput
content = content.replace(
    'pub struct DraftOutput {',
    '/// The output of a completed calibration draft session.\npub struct DraftOutput {'
)
content = content.replace(
    'pub profile_path: PathBuf,',
    '/// Path to the generated draft TOML profile.\n    pub profile_path: PathBuf,'
)
content = content.replace(
    'pub report_path: PathBuf,',
    '/// Path to the detailed calibration JSON report.\n    pub report_path: PathBuf,'
)

# Add docs for CalibrationRecorder
content = content.replace(
    'pub struct CalibrationRecorder {',
    '/// Automated tool that generates `RtaProfile` draft rules from a player\'s manual split inputs by analyzing RAM state changes.\npub struct CalibrationRecorder {'
)


with open('crates/nes-desktop/src/rta.rs', 'w') as f:
    f.write(content)
