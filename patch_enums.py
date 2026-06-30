import re

with open('crates/nes-desktop/src/rta.rs', 'r') as f:
    content = f.read()

# Add docs for constants
content = content.replace(
    'pub const DEFAULT_RTA_PROFILES_DIR: &str = "config/rta/profiles";',
    '/// Default directory for loading profiles.\npub const DEFAULT_RTA_PROFILES_DIR: &str = "config/rta/profiles";'
)
content = content.replace(
    'pub const DEFAULT_RTA_RUNS_DIR: &str = "runs/rta";',
    '/// Default directory for saving runs.\npub const DEFAULT_RTA_RUNS_DIR: &str = "runs/rta";'
)

# Add docs for RtaRuntimeConfig
content = content.replace(
    'pub struct RtaRuntimeConfig {',
    '/// Configuration for the RTA session passed from CLI args.\npub struct RtaRuntimeConfig {'
)
content = re.sub(r'(\s+)pub profile_id_override: Option<String>,', r'\1/// Explicit profile ID to force load.\1pub profile_id_override: Option<String>,', content)
content = re.sub(r'(\s+)pub profiles_dir: PathBuf,', r'\1/// Directory to load profiles from.\1pub profiles_dir: PathBuf,', content)
content = re.sub(r'(\s+)pub runs_dir: PathBuf,', r'\1/// Directory to save run artifacts to.\1pub runs_dir: PathBuf,', content)
content = re.sub(r'(\s+)pub calibrate: bool,', r'\1/// Enable calibration draft mode.\1pub calibrate: bool,', content)

# Add docs for ProfileStatus
content = content.replace(
    'pub enum ProfileStatus {',
    '/// The official status of the profile.\npub enum ProfileStatus {'
)
content = re.sub(r'(\s+)Draft,', r'\1/// Profile is in draft and cannot be auto-selected for official runs.\1Draft,', content)
content = re.sub(r'(\s+)#\[default\]\n\s+Published,', r'\1#[default]\n\1/// Profile is official and ready to be played.\1Published,', content)

# Add docs for TimerClock
content = content.replace(
    'pub enum TimerClock {',
    '/// Represents the source of time measurement for the speedrun.\npub enum TimerClock {'
)
content = re.sub(r'(\s+)#\[default\]\n\s+Wall,', r'\1#[default]\n\1/// Measures absolute real-time (wall clock).\1Wall,', content)

# Add docs for FocusLossPolicy
content = content.replace(
    'pub enum FocusLossPolicy {',
    '/// How the timer behaves when the emulator window loses focus.\npub enum FocusLossPolicy {'
)
content = re.sub(r'(\s+)AutoPause,', r'\1/// Automatically pause the timer.\1AutoPause,', content)
content = re.sub(r'(\s+)Invalidate,', r'\1/// Instantly invalidate the run.\1Invalidate,', content)
content = re.sub(r'(\s+)#\[default\]\n\s+Continue,', r'\1#[default]\n\1/// Continue measuring time normally.\1Continue,', content)

# Add docs for ForbiddenAction
content = content.replace(
    'pub enum ForbiddenAction {',
    '/// Actions that, if performed during a run, can trigger an invalidation.\npub enum ForbiddenAction {'
)
content = re.sub(r'(\s+)#\[default\]\n\s+Rewind,', r'\1#[default]\n\1/// Using the emulator rewind feature.\1Rewind,', content)
content = re.sub(r'(\s+)SaveLoad,', r'\1/// Loading a manual save state.\1SaveLoad,', content)
content = re.sub(r'(\s+)FrameStep,', r'\1/// Using frame-by-frame stepping.\1FrameStep,', content)

# Add docs for TriggerWidth
content = content.replace(
    'pub enum TriggerWidth {',
    '/// The size of memory to read for a trigger condition.\npub enum TriggerWidth {'
)
content = re.sub(r'(\s+)#\[default\]\n\s+U8,', r'\1#[default]\n\1/// 1-byte read.\1U8,', content)
content = re.sub(r'(\s+)U16,', r'\1/// 2-byte read.\1U16,', content)
content = re.sub(r'(\s+)U32,', r'\1/// 4-byte read.\1U32,', content)

# Modify docs for TriggerOp
content = content.replace(
    'pub enum TriggerOp {',
    '/// The operation used to compare a memory value to a target.\npub enum TriggerOp {'
)
content = re.sub(r'(\s+)#\[default\]\n\s+Eq,', r'\1#[default]\n\1/// Trigger when value equals.\1Eq,', content)
content = re.sub(r'(\s+)Ne,', r'\1/// Trigger when value does not equal.\1Ne,', content)
content = re.sub(r'(\s+)Gt,', r'\1/// Trigger when value is strictly greater than.\1Gt,', content)
content = re.sub(r'(\s+)Gte,', r'\1/// Trigger when value is greater than or equal to.\1Gte,', content)
content = re.sub(r'(\s+)Lt,', r'\1/// Trigger when value is strictly less than.\1Lt,', content)
content = re.sub(r'(\s+)Lte,', r'\1/// Trigger when value is less than or equal to.\1Lte,', content)
content = re.sub(r'(\s+)BitSet,', r'\1/// Trigger when specific bits are set.\1BitSet,', content)
content = re.sub(r'(\s+)BitClear,', r'\1/// Trigger when specific bits are clear.\1BitClear,', content)
content = re.sub(r'(\s+)Changed,', r'\1/// Trigger when value changes.\1Changed,', content)


with open('crates/nes-desktop/src/rta.rs', 'w') as f:
    f.write(content)
