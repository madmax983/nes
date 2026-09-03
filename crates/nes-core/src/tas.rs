//! Stable TAS primitives for recording and replaying deterministic input movies.

use alloc::{string::String, vec::Vec};

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

impl core::error::Error for TasError {}

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tas_movie_to_macro_script_fails_with_player_2_input() {
        let mut movie = TasMovie::default();
        movie.push_run(TasFrameRun::new(0, 1, 10)); // controller2_bits != 0
        let result = movie.to_macro_script();
        assert_eq!(result, Err(TasError::Player2MacroScriptUnsupported));
    }

    #[test]
    fn test_tas_movie_push_run_coalescing() {
        let mut movie = TasMovie::default();

        // Zero frames run should be ignored
        movie.push_run(TasFrameRun::new(0, 0, 0));
        assert!(movie.runs().is_empty());

        // Initial run
        movie.push_run(TasFrameRun::new(0, 0, 10));
        assert_eq!(movie.runs(), &[TasFrameRun::new(0, 0, 10)]);

        // Coalescing same inputs
        movie.push_run(TasFrameRun::new(0, 0, 5));
        assert_eq!(movie.runs(), &[TasFrameRun::new(0, 0, 15)]);

        // Different controller 1 input
        movie.push_run(TasFrameRun::new(1, 0, 10));
        assert_eq!(
            movie.runs(),
            &[TasFrameRun::new(0, 0, 15), TasFrameRun::new(1, 0, 10)]
        );

        // Different controller 2 input
        movie.push_run(TasFrameRun::new(1, 1, 10));
        assert_eq!(
            movie.runs(),
            &[
                TasFrameRun::new(0, 0, 15),
                TasFrameRun::new(1, 0, 10),
                TasFrameRun::new(1, 1, 10)
            ]
        );

        // Frame limit overflow prevents coalescing
        let mut runs = movie.runs().to_vec();
        runs.last_mut().unwrap().frames = u32::MAX;
        let mut max_movie = TasMovie::from_runs(runs);

        max_movie.push_run(TasFrameRun::new(1, 1, 1));
        assert_eq!(
            max_movie.runs(),
            &[
                TasFrameRun::new(0, 0, 15),
                TasFrameRun::new(1, 0, 10),
                TasFrameRun::new(1, 1, u32::MAX),
                TasFrameRun::new(1, 1, 1)
            ]
        );
    }
}

#[cfg(test)]
mod tests_mutants {
    use super::*;
    use crate::NesCore;

    fn looping_core() -> NesCore {
        let mut core = NesCore::new();
        core.load_cpu_bytes(0xC000, &[0xEA, 0x4C, 0x00, 0xC0]);
        core
    }

    #[test]
    fn test_tas_frame_run_new() {
        let run = TasFrameRun::new(1, 2, 3);
        assert_eq!(run.controller1_bits, 1);
        assert_eq!(run.controller2_bits, 2);
        assert_eq!(run.frames, 3);
    }

    #[test]
    fn test_tas_movie_methods() {
        let mut movie = TasMovie::default();
        assert_eq!(movie.total_frames(), 0);

        movie = TasMovie::from_runs(vec![TasFrameRun::new(1, 2, 5)]);
        assert_eq!(movie.total_frames(), 5);

        let runs = movie.runs();
        assert_eq!(runs.len(), 1);
        assert_eq!(runs[0], TasFrameRun::new(1, 2, 5));

        let mut core = looping_core();
        let frames = movie.replay(&mut core).unwrap();
        assert_eq!(frames, 5);

        movie.push_frame(3, 4);
        assert_eq!(movie.runs().len(), 2);
    }

    #[test]
    fn test_tas_recorder_methods() {
        let mut recorder = TasRecorder::default();
        assert!(!recorder.is_recording());

        recorder.start();
        assert!(recorder.is_recording());

        recorder.record_frame(1);

        recorder.stop();
        assert!(!recorder.is_recording());

        recorder.start();
        recorder.record_frame_bits(1, 2);

        recorder.clear();
        assert!(recorder.is_recording());

        let core = NesCore::new();
        recorder.record_core_frame(&core);

        let movie = recorder.finish();
        let _ = movie.runs();
    }
}
