import re

with open('crates/nes-desktop/src/rta.rs', 'r') as f:
    content = f.read()

# Add docs for TimerPolicy
content = content.replace(
    'pub struct TimerPolicy {',
    '/// Rules regarding how the timer measures time.\npub struct TimerPolicy {'
)
content = re.sub(
    r'(\s+)pub clock: TimerClock,',
    r'\1/// The time source for the run.\1pub clock: TimerClock,',
    content
)
content = re.sub(
    r'(\s+)pub focus_loss: FocusLossPolicy,',
    r'\1/// How the timer behaves when the application loses focus.\1pub focus_loss: FocusLossPolicy,',
    content
)
content = re.sub(
    r'(\s+)pub manual_fallback: bool,',
    r'\1/// Whether the runner can manually manipulate the timer.\1pub manual_fallback: bool,',
    content
)

# Add docs for InvalidationPolicy
content = content.replace(
    'pub struct InvalidationPolicy {',
    '/// Rules dictating which actions invalidate the run.\npub struct InvalidationPolicy {'
)
content = re.sub(
    r'(\s+)pub invalidate_on: Vec<ForbiddenAction>,',
    r'\1/// List of actions that invalidate the run.\1pub invalidate_on: Vec<ForbiddenAction>,',
    content
)

# Add docs for SplitPolicy
content = content.replace(
    'pub struct SplitPolicy {',
    '/// Rules defining how splits are recorded.\npub struct SplitPolicy {'
)
content = re.sub(
    r'(\s+)pub append_only: bool,',
    r'\1/// Whether to only allow adding new splits without overriding previous ones.\1pub append_only: bool,',
    content
)
content = re.sub(
    r'(\s+)pub manual_hotkey: String,',
    r'\1/// Hotkey used to trigger a manual split.\1pub manual_hotkey: String,',
    content
)

# Add docs for LoggingPolicy
content = content.replace(
    'pub struct LoggingPolicy {',
    '/// Configuration for run output artifacts.\npub struct LoggingPolicy {'
)
content = re.sub(
    r'(\s+)pub save_input_log: bool,',
    r'\1/// Whether to save detailed input logs per frame.\1pub save_input_log: bool,',
    content
)

# Add docs for TriggerRule
content = content.replace(
    'pub struct TriggerRule {',
    '/// A rule triggering an RTA event based on memory states.\npub struct TriggerRule {'
)
content = re.sub(
    r'(\s+)pub address: u16,',
    r'\1/// Memory address to monitor.\1pub address: u16,',
    content
)
content = re.sub(
    r'(\s+)pub width: TriggerWidth,',
    r'\1/// Number of bytes to read.\1pub width: TriggerWidth,',
    content
)
content = re.sub(
    r'(\s+)pub op: TriggerOp,',
    r'\1/// Comparison operation to perform against `value`.\1pub op: TriggerOp,',
    content
)
content = re.sub(
    r'(\s+)pub value: u32,',
    r'\1/// Value to compare against.\1pub value: u32,',
    content
)
content = re.sub(
    r'(\s+)pub debounce_frames: u32,',
    r'\1/// Frames to wait before re-triggering.\1pub debounce_frames: u32,',
    content
)
content = re.sub(
    r'(\s+)pub require_consecutive: u32,',
    r'\1/// Frames the condition must consecutively hold to trigger.\1pub require_consecutive: u32,',
    content
)

# Add docs for LoadedProfile
content = content.replace(
    'pub struct LoadedProfile {',
    '/// A profile loaded from disk along with its path.\npub struct LoadedProfile {'
)
content = re.sub(
    r'(\s+)pub path: PathBuf,',
    r'\1/// File path to the profile.\1pub path: PathBuf,',
    content
)
content = re.sub(
    r'(\s+)pub profile: RtaProfile,',
    r'\1/// The profile contents.\1pub profile: RtaProfile,',
    content
)

# Add docs for ProfileSelectionSource
content = content.replace(
    'pub enum ProfileSelectionSource {',
    '/// Source of the profile selection.\npub enum ProfileSelectionSource {'
)
content = re.sub(
    r'(\s+)AutoByRomHash,',
    r'\1/// Automatically selected by matching ROM hash.\1AutoByRomHash,',
    content
)
content = re.sub(
    r'(\s+)ManualOverride,',
    r'\1/// Manually overridden by the user.\1ManualOverride,',
    content
)

# Add docs for ProfileSelection
content = content.replace(
    'pub struct ProfileSelection {',
    '/// The final selected profile and its selection source.\npub struct ProfileSelection {'
)
content = re.sub(
    r'(\s+)pub selected: LoadedProfile,',
    r'\1/// The selected profile.\1pub selected: LoadedProfile,',
    content
)
content = re.sub(
    r'(\s+)pub source: ProfileSelectionSource,',
    r'\1/// How the profile was selected.\1pub source: ProfileSelectionSource,',
    content
)

# Add docs for RtaSessionState
content = content.replace(
    'pub enum RtaSessionState {',
    '/// Current state of the RTA session.\npub enum RtaSessionState {'
)
content = re.sub(
    r'(\s+)Idle,',
    r'\1/// Waiting for start condition.\1Idle,',
    content,
    count=1
)
content = re.sub(
    r'(\s+)Armed,',
    r'\1/// Ready and armed to start.\1Armed,',
    content,
    count=1
)
content = re.sub(
    r'(\s+)Running,',
    r'\1/// Actively recording a run.\1Running,',
    content,
    count=1
)
content = re.sub(
    r'(\s+)Finished,',
    r'\1/// Run is completed.\1Finished,',
    content,
    count=1
)
content = re.sub(
    r'(\s+)InvalidPractice,',
    r'\1/// Run was invalidated.\1InvalidPractice,',
    content,
    count=1
)

# Add docs for SplitSource
content = content.replace(
    'pub enum SplitSource {',
    '/// How a split was triggered.\npub enum SplitSource {'
)
content = re.sub(
    r'(\s+)Automatic,',
    r'\1/// Automatically by a memory trigger.\1Automatic,',
    content,
    count=1
)
content = re.sub(
    r'(\s+)Manual,',
    r'\1/// Manually by the user via a hotkey.\1Manual,',
    content,
    count=1
)

# Add docs for SplitEvent
content = content.replace(
    'pub struct SplitEvent {',
    '/// A recorded split event.\npub struct SplitEvent {'
)
content = re.sub(
    r'(\s+)pub name: String,',
    r'\1/// Name of the split.\1pub name: String,',
    content
)
content = re.sub(
    r'(\s+)pub source: SplitSource,',
    r'\1/// How the split was triggered.\1pub source: SplitSource,',
    content
)
content = re.sub(
    r'(\s+)pub frame: u64,',
    r'\1/// Frame number when the split occurred.\1pub frame: u64,',
    content
)
content = re.sub(
    r'(\s+)pub elapsed_ms: u128,',
    r'\1/// Elapsed time in milliseconds.\1pub elapsed_ms: u128,',
    content
)

# Add docs for InputLogFrame
content = content.replace(
    'pub struct InputLogFrame {',
    '/// A single frame of input logging.\npub struct InputLogFrame {'
)
content = re.sub(
    r'(\s+)pub controller1_bits: u8,',
    r'\1/// Controller 1 input state bits.\1pub controller1_bits: u8,',
    content
)
content = re.sub(
    r'(\s+)pub controller2_bits: u8,',
    r'\1/// Controller 2 input state bits.\1pub controller2_bits: u8,',
    content
)

# Add docs for RtaEvent
content = content.replace(
    'pub enum RtaEvent {',
    '/// Events emitted by the RTA session.\npub enum RtaEvent {'
)
content = re.sub(
    r'(\s+)Started,',
    r'\1/// Run started.\1Started,',
    content,
    count=1
)
content = re.sub(
    r'(\s+)Paused,',
    r'\1/// Run paused.\1Paused,',
    content,
    count=1
)
content = re.sub(
    r'(\s+)Resumed,',
    r'\1/// Run resumed.\1Resumed,',
    content,
    count=1
)
content = re.sub(
    r'(\s+)Split\(SplitEvent\),',
    r'\1/// A split occurred.\1Split(SplitEvent),',
    content,
    count=1
)
content = re.sub(
    r'(\s+)Invalidated\(String\),',
    r'\1/// Run was invalidated with reason.\1Invalidated(String),',
    content,
    count=1
)
content = re.sub(
    r'(\s+)Finished\(Duration\),',
    r'\1/// Run finished successfully with total duration.\1Finished(Duration),',
    content,
    count=1
)


# Add docs for RunArtifactPaths
content = content.replace(
    'pub struct RunArtifactPaths {',
    '/// Paths to the generated output artifacts after a run.\npub struct RunArtifactPaths {'
)
content = re.sub(
    r'(\s+)pub run_json_path: PathBuf,',
    r'\1/// Path to the JSON run file.\1pub run_json_path: PathBuf,',
    content
)
content = re.sub(
    r'(\s+)pub input_log_path: Option<PathBuf>,',
    r'\1/// Path to the optional input log file.\1pub input_log_path: Option<PathBuf>,',
    content
)

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
content = re.sub(
    r'(\s+)pub profile_path: PathBuf,',
    r'\1/// Path to the generated draft TOML profile.\1pub profile_path: PathBuf,',
    content
)
content = re.sub(
    r'(\s+)pub report_path: PathBuf,',
    r'\1/// Path to the detailed calibration JSON report.\1pub report_path: PathBuf,',
    content
)

# Add docs for CalibrationRecorder
content = content.replace(
    'pub struct CalibrationRecorder {',
    '/// Automated tool that generates `RtaProfile` draft rules from a player\'s manual split inputs by analyzing RAM state changes.\npub struct CalibrationRecorder {'
)


with open('crates/nes-desktop/src/rta.rs', 'w') as f:
    f.write(content)
