1. **Add Missing Documentation to `crates/nes-desktop/src/rta.rs`**: Use `run_in_bash_session` to embed a python script (via heredoc) that uses targeted multiline string replacements to inject `///` comments directly into `crates/nes-desktop/src/rta.rs`.
```bash
cat << 'PYEOF' > replace.py
import re

with open('crates/nes-desktop/src/rta.rs', 'r') as f:
    content = f.read()

replacements = [
    (
        "pub struct TimerPolicy {",
        "/// Defines how the speedrun timer behaves under different runtime conditions.\n///\n/// Speedruns require a stable clock. This policy determines if the timer pauses when the emulator loses focus,\n/// what hardware clock to use, and whether manual start/stop fallback is allowed if automatic memory triggers fail.\n///\n/// ## Examples\n/// ```no_run\n/// use nes_desktop::rta::{TimerPolicy, TimerClock, FocusLossPolicy};\n/// let policy = TimerPolicy {\n///     clock: TimerClock::Wall,\n///     focus_loss: FocusLossPolicy::Continue,\n///     manual_fallback: false,\n/// };\n/// ```\npub struct TimerPolicy {"
    ),
    (
        "pub clock: TimerClock,",
        "/// The source of truth for time tracking (e.g. Wall clock vs Emulation Frames).\n    pub clock: TimerClock,"
    ),
    (
        "pub focus_loss: FocusLossPolicy,",
        "/// What to do if the emulator window loses focus (e.g. pause timer or continue).\n    pub focus_loss: FocusLossPolicy,"
    ),
    (
        "pub manual_fallback: bool,",
        "/// Whether the runner is allowed to manually start/stop the timer via hotkeys if auto-triggers fail.\n    pub manual_fallback: bool,"
    ),
    (
        "pub struct InvalidationPolicy {",
        "/// Strict rules that define what actions instantly disqualify a speedrun attempt.\n///\n/// To ensure competitive integrity, certain emulator features like rewinding or loading save states\n/// must be forbidden during an active run. If any of these actions are detected, the run is immediately marked as invalid.\n///\n/// ## Examples\n/// ```no_run\n/// use nes_desktop::rta::{InvalidationPolicy, ForbiddenAction};\n/// let policy = InvalidationPolicy {\n///     invalidate_on: vec![ForbiddenAction::Rewind, ForbiddenAction::SaveLoad],\n/// };\n/// ```\npub struct InvalidationPolicy {"
    ),
    (
        "pub invalidate_on: Vec<ForbiddenAction>,",
        "/// A list of specific emulator features that will instantly invalidate the run if used.\n    pub invalidate_on: Vec<ForbiddenAction>,"
    ),
    (
        "pub struct SplitPolicy {",
        "/// Determines how the split sequence is managed and whether the runner can intervene manually.\n///\n/// In many categories, splits must follow a strict, append-only order to prevent runners from skipping segments.\n/// This policy also configures the hotkey for manual splits if the category allows it.\n///\n/// ## Examples\n/// ```no_run\n/// use nes_desktop::rta::SplitPolicy;\n/// let policy = SplitPolicy {\n///     append_only: true,\n///     manual_hotkey: \"F9\".to_owned(),\n/// };\n/// ```\npub struct SplitPolicy {"
    ),
    (
        "pub append_only: bool,",
        "/// If true, new splits can only be added sequentially. Existing splits cannot be deleted or re-ordered during the run.\n    pub append_only: bool,"
    ),
    (
        "pub manual_hotkey: String,",
        "/// The keyboard hotkey binding used to manually trigger a split event.\n    pub manual_hotkey: String,"
    ),
    (
        "pub struct LoggingPolicy {",
        "/// Configures what artifact data is written to disk during and after a speedrun.\n///\n/// Artifacts like input logs are critical for post-run verification and cheat detection.\n/// This policy dictates whether every frame's controller input is recorded to a `.run.json` file.\n///\n/// ## Examples\n/// ```no_run\n/// use nes_desktop::rta::LoggingPolicy;\n/// let policy = LoggingPolicy {\n///     save_input_log: true,\n/// };\n/// ```\npub struct LoggingPolicy {"
    ),
    (
        "pub save_input_log: bool,",
        "/// If true, records a frame-by-frame log of all controller inputs during the active run.\n    pub save_input_log: bool,"
    ),
    (
        "pub struct TriggerRule {",
        "/// A memory inspection rule used to automatically trigger events like starting the timer or splitting.\n///\n/// Emulators have the unique advantage of peeking directly into the game's RAM.\n/// A `TriggerRule` watches a specific memory address, reads a value of a certain width, and applies an operation\n/// (like `Eq` or `Changed`) to determine if a speedrun event should occur.\n///\n/// ## Examples\n/// ```no_run\n/// use nes_desktop::rta::{TriggerRule, TriggerWidth, TriggerOp};\n/// let rule = TriggerRule {\n///     address: 0x071A, // e.g. SMB world number\n///     width: TriggerWidth::U8,\n///     op: TriggerOp::Eq,\n///     value: 1,\n///     debounce_frames: 0,\n///     require_consecutive: 1,\n/// };\n/// ```\npub struct TriggerRule {"
    ),
    (
        "pub address: u16,",
        "/// The CPU memory bus address to inspect.\n    pub address: u16,"
    ),
    (
        "pub width: TriggerWidth,",
        "/// The size of the memory value to read (e.g. 8-bit or 16-bit).\n    pub width: TriggerWidth,"
    ),
    (
        "pub op: TriggerOp,",
        "/// The logical operation to perform on the read value (e.g. `Eq`, `GreaterThan`, `BitSet`).\n    pub op: TriggerOp,"
    ),
    (
        "pub value: u32,",
        "/// The target value or bitmask to compare against.\n    pub value: u32,"
    ),
    (
        "pub debounce_frames: u32,",
        "/// The number of frames to wait after a trigger before it can be activated again.\n    pub debounce_frames: u32,"
    ),
    (
        "pub require_consecutive: u32,",
        "/// The condition must be true for this many consecutive frames before triggering.\n    pub require_consecutive: u32,"
    )
]

for search, replace in replacements:
    content = content.replace(search, replace)

with open('crates/nes-desktop/src/rta.rs', 'w') as f:
    f.write(content)

PYEOF
python3 replace.py
rm replace.py
```
2. **Verify Documentation**: Run `RUSTDOCFLAGS="-D missing_docs" cargo doc --no-deps --all-features` to verify that the missing docs for the documented items have been resolved.
3. **Run Code Quality Checks**: Use `run_in_bash_session` to run `cargo fmt --all`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, and `cargo test --workspace --all-targets --all-features` to ensure that no regressions or warnings are introduced.
4. **Pre-commit steps**: Complete pre-commit steps to ensure proper testing, verification, review, and reflection are done.
5. **Submit PR**: Submit a PR targeting branch `main` with the title '🎻 Bard: [documentation update]' and the precise description sections "📖 Chapter", "🔦 Insight", "🧪 Example", "🖼️ Preview".
