//! Stable TAS primitives for recording and replaying deterministic input movies.

use core::fmt;

use serde::{Deserialize, Serialize};

use crate::{Button, Command, CoreError, NesCore};

const BUTTON_NAMES: [(Button, &str); 8] = [
    (Button::A, "A"),
    (Button::B, "B"),
    (Button::Select, "Select"),
    (Button::Start, "Start"),
    (Button::Up, "Up"),
    (Button::Down, "Down"),
    (Button::Left, "Left"),
    (Button::Right, "Right"),
];

/// Errors raised by TAS serialization or compatibility helpers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TasError {
    /// Legacy macro scripts only model player 1 and cannot encode player 2 input.
    Player2MacroScriptUnsupported,
}

impl fmt::Display for TasError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Player2MacroScriptUnsupported => {
                f.write_str("legacy macro scripts do not support player 2 input")
            }
        }
    }
}

impl std::error::Error for TasError {}

/// Run-length encoded controller state for consecutive emulator frames.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct TasFrameRun {
    /// Controller 1 bitfield applied while this run is active.
    pub controller1_bits: u8,
    /// Controller 2 bitfield applied while this run is active.
    pub controller2_bits: u8,
    /// Number of consecutive frames using the same controller state.
    pub frames: u32,
}

impl TasFrameRun {
    /// Creates a run covering `frames` frames with both controller bitfields.
    #[must_use]
    pub const fn new(controller1_bits: u8, controller2_bits: u8, frames: u32) -> Self {
        Self {
            controller1_bits,
            controller2_bits,
            frames,
        }
    }
}

/// Deterministic TAS movie represented as coalesced controller-state runs.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct TasMovie {
    runs: Vec<TasFrameRun>,
}

impl TasMovie {
    /// Builds a movie from controller-state runs, coalescing adjacent identical runs.
    #[must_use]
    pub fn from_runs(runs: Vec<TasFrameRun>) -> Self {
        let mut movie = Self::default();
        for run in runs {
            movie.push_run(run);
        }
        movie
    }

    /// Returns the movie's coalesced controller-state runs.
    #[must_use]
    pub fn runs(&self) -> &[TasFrameRun] {
        &self.runs
    }

    /// Returns the total number of frames represented by this movie.
    #[must_use]
    pub fn total_frames(&self) -> u64 {
        self.runs.iter().map(|run| u64::from(run.frames)).sum()
    }

    /// Replays the full movie onto `core` using stable controller commands.
    pub fn replay(&self, core: &mut NesCore) -> Result<u64, CoreError> {
        let mut frames_elapsed = 0_u64;
        for run in &self.runs {
            if run.frames == 0 {
                continue;
            }
            core.execute(Command::SetControllerState(run.controller1_bits))?;
            core.execute(Command::SetController2State(run.controller2_bits))?;
            for _ in 0..run.frames {
                core.execute(Command::StepFrame)?;
                frames_elapsed += 1;
            }
        }
        Ok(frames_elapsed)
    }

    /// Exports the movie to the legacy line-based macro script format.
    pub fn to_macro_script(&self) -> Result<String, TasError> {
        let mut script = String::new();
        let mut previous_bits = 0_u8;
        for run in &self.runs {
            if run.frames == 0 {
                continue;
            }
            if run.controller2_bits != 0 {
                return Err(TasError::Player2MacroScriptUnsupported);
            }
            append_button_transitions(&mut script, previous_bits, run.controller1_bits);
            append_wait(&mut script, run.frames);
            previous_bits = run.controller1_bits;
        }
        Ok(script)
    }

    fn push_frame(&mut self, controller1_bits: u8, controller2_bits: u8) {
        self.push_run(TasFrameRun::new(controller1_bits, controller2_bits, 1));
    }

    fn push_run(&mut self, run: TasFrameRun) {
        if run.frames == 0 {
            return;
        }
        if let Some(last) = self.runs.last_mut()
            && last.controller1_bits == run.controller1_bits
            && last.controller2_bits == run.controller2_bits
            && let Some(total_frames) = last.frames.checked_add(run.frames)
        {
            last.frames = total_frames;
            return;
        }
        self.runs.push(run);
    }
}

/// Recorder that captures per-frame controller state into a stable [`TasMovie`].
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TasRecorder {
    recording: bool,
    movie: TasMovie,
}

impl TasRecorder {
    /// Creates a new recorder with no captured frames.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Starts or resumes recording.
    pub fn start(&mut self) {
        self.recording = true;
    }

    /// Stops recording without discarding captured frames.
    pub fn stop(&mut self) {
        self.recording = false;
    }

    /// Returns whether the recorder is currently active.
    #[must_use]
    pub fn is_recording(&self) -> bool {
        self.recording
    }

    /// Clears the captured movie while preserving the current recording state.
    pub fn clear(&mut self) {
        self.movie = TasMovie::default();
    }

    /// Records one frame of player-1 input for compatibility with the legacy recorder.
    pub fn record_frame(&mut self, controller_bits: u8) {
        self.record_frame_bits(controller_bits, 0);
    }

    /// Records one frame of controller input for both players.
    pub fn record_frame_bits(&mut self, controller1_bits: u8, controller2_bits: u8) {
        if !self.recording {
            return;
        }
        self.movie.push_frame(controller1_bits, controller2_bits);
    }

    /// Records one frame using the current controller state from a live [`NesCore`].
    pub fn record_core_frame(&mut self, core: &NesCore) {
        self.record_frame_bits(core.controller_bits(), core.controller2_bits());
    }

    /// Returns the captured movie.
    #[must_use]
    pub fn movie(&self) -> &TasMovie {
        &self.movie
    }

    /// Consumes the recorder and returns the captured movie.
    #[must_use]
    pub fn finish(self) -> TasMovie {
        self.movie
    }

    /// Exports the current recording to the legacy line-based macro script format.
    pub fn macro_script(&self) -> Result<String, TasError> {
        self.movie.to_macro_script()
    }
}

/// Backward-compatible name for recorder users who still think in "macro recorder" terms.
pub type MacroRecorder = TasRecorder;

fn append_button_transitions(script: &mut String, previous_bits: u8, current_bits: u8) {
    let pressed = current_bits & !previous_bits;
    let released = !current_bits & previous_bits;
    for (button, name) in BUTTON_NAMES {
        let mask = button.bit_mask();
        if pressed & mask != 0 {
            script.push_str("PRESS ");
            script.push_str(name);
            script.push('\n');
        } else if released & mask != 0 {
            script.push_str("RELEASE ");
            script.push_str(name);
            script.push('\n');
        }
    }
}

fn append_wait(script: &mut String, frames: u32) {
    script.push_str("WAIT ");
    let mut buf = itoa::Buffer::new();
    script.push_str(buf.format(frames));
    script.push('\n');
}
