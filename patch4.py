import re

with open('crates/nes-desktop/src/rta.rs', 'r') as f:
    content = f.read()

# Add docs for SplitSource
content = content.replace(
    'pub enum SplitSource {',
    '/// How a split was triggered.\npub enum SplitSource {'
)
content = content.replace(
    '    Automatic,\n',
    '    /// Automatically by a memory trigger.\n    Automatic,\n'
)
content = content.replace(
    '    Manual,\n',
    '    /// Manually by the user via a hotkey.\n    Manual,\n'
)

# Add docs for SplitEvent
content = content.replace(
    'pub struct SplitEvent {',
    '/// A recorded split event.\npub struct SplitEvent {'
)
content = content.replace(
    'pub name: String,',
    '/// Name of the split.\n    pub name: String,'
)
content = content.replace(
    'pub source: SplitSource,',
    '/// How the split was triggered.\n    pub source: SplitSource,'
)
content = content.replace(
    'pub frame: u64,',
    '/// Frame number when the split occurred.\n    pub frame: u64,'
)
content = content.replace(
    'pub elapsed_ms: u128,',
    '/// Elapsed time in milliseconds.\n    pub elapsed_ms: u128,'
)

# Add docs for InputLogFrame
content = content.replace(
    'pub struct InputLogFrame {',
    '/// A single frame of input logging.\npub struct InputLogFrame {'
)
content = content.replace(
    'pub frame: u64,\n',
    '/// Frame number.\n    pub frame: u64,\n'
)
content = content.replace(
    'pub controller1_bits: u8,',
    '/// Controller 1 input state bits.\n    pub controller1_bits: u8,'
)
content = content.replace(
    'pub controller2_bits: u8,',
    '/// Controller 2 input state bits.\n    pub controller2_bits: u8,'
)
content = content.replace(
    'pub elapsed_ms: u128,\n',
    '/// Elapsed time in milliseconds.\n    pub elapsed_ms: u128,\n'
)

# Add docs for RtaEvent
content = content.replace(
    'pub enum RtaEvent {',
    '/// Events emitted by the RTA session.\npub enum RtaEvent {'
)
content = content.replace(
    '    Started,\n',
    '    /// Run started.\n    Started,\n'
)
content = content.replace(
    '    Paused,\n',
    '    /// Run paused.\n    Paused,\n'
)
content = content.replace(
    '    Resumed,\n',
    '    /// Run resumed.\n    Resumed,\n'
)
content = content.replace(
    '    Split(SplitEvent),\n',
    '    /// A split occurred.\n    Split(SplitEvent),\n'
)
content = content.replace(
    '    Invalidated(String),\n',
    '    /// Run was invalidated with reason.\n    Invalidated(String),\n'
)
content = content.replace(
    '    Finished(Duration),\n',
    '    /// Run finished successfully with total duration.\n    Finished(Duration),\n'
)

# Add docs for RunArtifactPaths
content = content.replace(
    'pub struct RunArtifactPaths {',
    '/// Paths to the generated output artifacts after a run.\npub struct RunArtifactPaths {'
)
content = content.replace(
    'pub run_json_path: PathBuf,',
    '/// Path to the JSON run file.\n    pub run_json_path: PathBuf,'
)
content = content.replace(
    'pub input_log_path: Option<PathBuf>,',
    '/// Path to the optional input log file.\n    pub input_log_path: Option<PathBuf>,'
)

with open('crates/nes-desktop/src/rta.rs', 'w') as f:
    f.write(content)
