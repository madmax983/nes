import re

with open('crates/nes-desktop/src/rta.rs', 'r') as f:
    content = f.read()

# Add docs for TimerPolicy
content = content.replace(
    'pub struct TimerPolicy {',
    '/// Rules regarding how the timer measures time.\npub struct TimerPolicy {'
)
content = content.replace(
    'pub clock: TimerClock,',
    '/// The time source for the run.\n    pub clock: TimerClock,'
)
content = content.replace(
    'pub focus_loss: FocusLossPolicy,',
    '/// How the timer behaves when the application loses focus.\n    pub focus_loss: FocusLossPolicy,'
)
content = content.replace(
    'pub manual_fallback: bool,',
    '/// Whether the runner can manually manipulate the timer.\n    pub manual_fallback: bool,'
)

# Add docs for InvalidationPolicy
content = content.replace(
    'pub struct InvalidationPolicy {',
    '/// Rules dictating which actions invalidate the run.\npub struct InvalidationPolicy {'
)
content = content.replace(
    'pub invalidate_on: Vec<ForbiddenAction>,',
    '/// List of actions that invalidate the run.\n    pub invalidate_on: Vec<ForbiddenAction>,'
)

# Add docs for SplitPolicy
content = content.replace(
    'pub struct SplitPolicy {',
    '/// Rules defining how splits are recorded.\npub struct SplitPolicy {'
)
content = content.replace(
    'pub append_only: bool,',
    '/// Whether to only allow adding new splits without overriding previous ones.\n    pub append_only: bool,'
)
content = content.replace(
    'pub manual_hotkey: String,',
    '/// Hotkey used to trigger a manual split.\n    pub manual_hotkey: String,'
)

# Add docs for LoggingPolicy
content = content.replace(
    'pub struct LoggingPolicy {',
    '/// Configuration for run output artifacts.\npub struct LoggingPolicy {'
)
content = content.replace(
    'pub save_input_log: bool,',
    '/// Whether to save detailed input logs per frame.\n    pub save_input_log: bool,'
)

# Add docs for TriggerRule
content = content.replace(
    'pub struct TriggerRule {',
    '/// A rule triggering an RTA event based on memory states.\npub struct TriggerRule {'
)
content = content.replace(
    'pub address: u16,',
    '/// Memory address to monitor.\n    pub address: u16,'
)
content = content.replace(
    'pub width: TriggerWidth,',
    '/// Number of bytes to read.\n    pub width: TriggerWidth,'
)
content = content.replace(
    'pub op: TriggerOp,',
    '/// Comparison operation to perform against `value`.\n    pub op: TriggerOp,'
)
content = content.replace(
    'pub value: u32,',
    '/// Value to compare against.\n    pub value: u32,'
)
content = content.replace(
    'pub debounce_frames: u32,',
    '/// Frames to wait before re-triggering.\n    pub debounce_frames: u32,'
)
content = content.replace(
    'pub require_consecutive: u32,',
    '/// Frames the condition must consecutively hold to trigger.\n    pub require_consecutive: u32,'
)

with open('crates/nes-desktop/src/rta.rs', 'w') as f:
    f.write(content)
