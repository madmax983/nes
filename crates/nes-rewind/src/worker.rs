//! TimeMachine: host-facing rewind API backed by a background worker thread.
//!
//! The main emulation thread calls [`TimeMachine::record_frame`] each frame
//! (non-blocking — drops if the channel is full) and [`TimeMachine::rewind_step`]
//! during rewind. All timeline compression and reconstruction runs on a
//! dedicated background thread so the audio/video loop is never stalled.

use std::sync::mpsc::{self, Receiver, SyncSender};
use std::thread;
use std::time::Duration;

use nes_core::{CoreSnapshot, NesCore};

use crate::cursor::{RewindCursor, RewindSpeed};
use crate::policy::KeyframePolicy;
use crate::timeline::CompressedTimeline;

// ---------------------------------------------------------------------------
// Config
// ---------------------------------------------------------------------------

/// Configuration for the Time Machine rewind system.
#[derive(Debug, Clone)]
pub struct TimeMachineConfig {
    /// How many seconds of history to retain (default: 30).
    pub max_history_seconds: u32,
    /// Minimum frames between forced keyframes (default: 60).
    pub keyframe_base_interval: u64,
    /// Delta byte threshold that, combined with 3× EMA, triggers early keyframe.
    pub delta_spike_threshold: u32,
}

impl Default for TimeMachineConfig {
    fn default() -> Self {
        Self {
            max_history_seconds: 30,
            keyframe_base_interval: 60,
            delta_spike_threshold: 2048,
        }
    }
}

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

/// Observable state of the Time Machine.
#[derive(Debug, Clone, PartialEq)]
pub enum TimeMachineState {
    /// Actively recording frames.
    Recording,
    /// Rewinding history.
    Rewinding {
        /// Approximate history time remaining (seconds from frame 0).
        seconds_remaining: f32,
    },
    /// Rewind history exhausted.
    Exhausted,
}

// ---------------------------------------------------------------------------
// Worker protocol
// ---------------------------------------------------------------------------

enum WorkerMsg {
    Record {
        frame_id: u64,
        snapshot: Box<CoreSnapshot>,
    },
    Reconstruct {
        target_frame: u64,
    },
    Shutdown,
}

enum WorkerReply {
    Reconstructed {
        frame_id: u64,
        snapshot: CoreSnapshot,
    },
}

// ---------------------------------------------------------------------------
// TimeMachine
// ---------------------------------------------------------------------------

/// Host-facing rewind controller.
///
/// # Usage
///
/// ```no_run
/// use nes_core::{Command, NesCore};
/// use nes_rewind::worker::{TimeMachine, TimeMachineConfig};
///
/// let mut core = NesCore::new();
/// let mut tm = TimeMachine::new(TimeMachineConfig::default());
///
/// // Each frame: advance then record.
/// core.execute(Command::StepFrame).unwrap();
/// tm.record_frame(&core);
///
/// // On rewind key: step backward.
/// tm.rewind_step(&mut core);
/// ```
pub struct TimeMachine {
    #[allow(dead_code)]
    config: TimeMachineConfig,
    state: TimeMachineState,
    cursor: Option<RewindCursor>,
    tx: SyncSender<WorkerMsg>,
    rx: Receiver<WorkerReply>,
    last_recorded_frame: u64,
}

impl TimeMachine {
    /// Construct a new `TimeMachine` and spawn its background worker thread.
    pub fn new(config: TimeMachineConfig) -> Self {
        let max_frames = u64::from(config.max_history_seconds) * 60;
        let policy =
            KeyframePolicy::new(config.keyframe_base_interval, config.delta_spike_threshold);

        // Channel capacities:
        //   work_tx capacity 4  — small; emulation thread drops rather than blocks.
        //   reply_tx capacity 64 — large enough for lookahead pre-fetch bursts.
        let (work_tx, work_rx) = mpsc::sync_channel::<WorkerMsg>(4);
        let (reply_tx, reply_rx) = mpsc::sync_channel::<WorkerReply>(64);

        thread::spawn(move || {
            let mut timeline = CompressedTimeline::new(max_frames, policy);
            loop {
                match work_rx.recv() {
                    Ok(WorkerMsg::Record { frame_id, snapshot }) => {
                        timeline.push(frame_id, *snapshot);
                    }
                    Ok(WorkerMsg::Reconstruct { target_frame }) => {
                        if let Some(snapshot) = timeline.reconstruct(target_frame) {
                            let _ = reply_tx.send(WorkerReply::Reconstructed {
                                frame_id: target_frame,
                                snapshot,
                            });
                        }
                    }
                    Ok(WorkerMsg::Shutdown) | Err(_) => break,
                }
            }
        });

        Self {
            config,
            state: TimeMachineState::Recording,
            cursor: None,
            tx: work_tx,
            rx: reply_rx,
            last_recorded_frame: 0,
        }
    }

    /// Record the current emulator state. Non-blocking — drops if worker is busy.
    ///
    /// Call once per frame after advancing the core, only while [`TimeMachineState::Recording`].
    pub fn record_frame(&mut self, core: &NesCore) {
        if !matches!(self.state, TimeMachineState::Recording) {
            return;
        }
        let frame_id = core.ppu_frame_counter();
        let snapshot = core.save_state();
        self.last_recorded_frame = frame_id;
        // `try_send` — never block the emulation thread.
        let _ = self.tx.try_send(WorkerMsg::Record {
            frame_id,
            snapshot: Box::new(snapshot),
        });
    }

    /// Step backward one frame. Returns `Some(frame_id)` on success, `None` when history is exhausted.
    ///
    /// Blocks up to 16 ms waiting for the worker to reconstruct the target frame.
    pub fn rewind_step(&mut self, core: &mut NesCore) -> Option<u64> {
        // Initialise the cursor on first rewind call.
        if self.cursor.is_none() {
            self.state = TimeMachineState::Rewinding {
                seconds_remaining: 0.0,
            };
            self.cursor = Some(RewindCursor::new(
                self.last_recorded_frame,
                RewindSpeed::Normal,
            ));
        }

        let cursor = self.cursor.as_mut().unwrap();
        let target = cursor.current_frame.checked_sub(1)?;

        // Request the previous frame from the worker.
        // Use blocking send here — during rewind the emulation loop is paused so
        // it is acceptable to wait briefly for a slot (worker drains quickly).
        let _ = self.tx.send(WorkerMsg::Reconstruct {
            target_frame: target,
        });

        // Wait up to one frame-budget (16 ms) for the reply.
        if let Ok(WorkerReply::Reconstructed { frame_id, snapshot }) =
            self.rx.recv_timeout(Duration::from_millis(16))
        {
            cursor.current_frame = frame_id;
            core.load_state(&snapshot);
            self.state = TimeMachineState::Rewinding {
                seconds_remaining: frame_id as f32 / 60.0,
            };
            Some(frame_id)
        } else {
            self.state = TimeMachineState::Exhausted;
            None
        }
    }

    /// Accelerate rewind (tap → Normal → Fast → Faster).
    pub fn rewind_faster(&mut self) {
        if let Some(cursor) = &mut self.cursor {
            cursor.speed = match cursor.speed {
                RewindSpeed::Single | RewindSpeed::Normal => RewindSpeed::Fast,
                RewindSpeed::Fast => RewindSpeed::Faster,
                RewindSpeed::Faster => RewindSpeed::Faster,
            };
        }
    }

    /// Resume recording from the current rewind position.
    pub fn resume(&mut self) {
        self.cursor = None;
        self.state = TimeMachineState::Recording;
    }

    /// Current observable state.
    pub fn state(&self) -> TimeMachineState {
        self.state.clone()
    }

    /// Approximate seconds of history recorded so far.
    pub fn history_seconds(&self) -> f32 {
        self.last_recorded_frame as f32 / 60.0
    }
}

impl Drop for TimeMachine {
    fn drop(&mut self) {
        // Best-effort graceful shutdown of the worker thread.
        let _ = self.tx.send(WorkerMsg::Shutdown);
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use nes_core::{Command, NesCore};

    fn make_core() -> NesCore {
        let mut core = NesCore::new();
        core.load_cpu_bytes(0xC000, &[0xEA, 0x4C, 0x00, 0xC0]);
        core
    }

    fn config() -> TimeMachineConfig {
        TimeMachineConfig {
            max_history_seconds: 10,
            keyframe_base_interval: 30,
            delta_spike_threshold: 2048,
        }
    }

    /// A helper function to poll the TimeMachine until it processes all queued
    /// frames. Returns `true` if it synced successfully, or `false` if it timed out.
    /// This avoids flaky hardcoded sleep delays in tests.
    fn wait_for_sync(tm: &mut TimeMachine) -> bool {
        // Drain the queue completely OUTSIDE the polling cycle to prevent
        // inadvertently discarding asynchronous responses from previous iterations.
        while tm.rx.try_recv().is_ok() {}

        let start = std::time::Instant::now();
        // Give it up to 5000ms to process the queue in extremely slow CI environments.
        while start.elapsed() < Duration::from_millis(5000) {
            // To check if the worker has caught up, we can ask for a reconstruct
            // of the last recorded frame. If it succeeds, the worker has processed it.
            if tm.last_recorded_frame == 0 {
                return true;
            }

            let _ = tm.tx.send(WorkerMsg::Reconstruct {
                target_frame: tm.last_recorded_frame,
            });

            // We must loop on the receiver here, because the worker might have
            // queued up multiple responses (e.g. to previous `Reconstruct` requests)
            // before we started listening, or during our loop. We keep draining
            // until we see the one we want, or we timeout.
            let inner_start = std::time::Instant::now();
            while inner_start.elapsed() < Duration::from_millis(1500) {
                if let Ok(WorkerReply::Reconstructed { frame_id, .. }) =
                    tm.rx.recv_timeout(Duration::from_millis(250))
                {
                    // The test core isn't mutated until `tm.rewind_step` does it.
                    // But we just sent a raw message.
                    // Since we received the reply, the worker is caught up.
                    if frame_id == tm.last_recorded_frame {
                        return true;
                    }
                }
            }
            // If it failed or we didn't match, loop around and try again.
        }
        false
    }

    #[test]
    fn record_and_rewind_restores_earlier_frame() {
        let mut core = make_core();
        let mut tm = TimeMachine::new(config());

        // Record 60 frames.
        for _ in 0..60 {
            core.execute(Command::StepFrame).unwrap();
            tm.record_frame(&core);
        }
        let frame_before_rewind = core.ppu_frame_counter();

        // Flush the worker channel.
        assert!(wait_for_sync(&mut tm), "Worker thread failed to sync");

        // Rewind 30 frames.
        for _ in 0..30 {
            tm.rewind_step(&mut core);
        }
        let frame_after_rewind = core.ppu_frame_counter();

        assert!(
            frame_after_rewind < frame_before_rewind,
            "Expected rewind to restore earlier frame, got {} >= {}",
            frame_after_rewind,
            frame_before_rewind
        );
    }

    #[test]
    fn rewind_returns_none_when_no_history() {
        let mut core = make_core();
        let mut tm = TimeMachine::new(TimeMachineConfig::default());

        // No frames recorded — first rewind_step should return None.
        let result = tm.rewind_step(&mut core);
        assert!(result.is_none());
    }

    #[test]
    fn resume_after_rewind_puts_machine_back_in_recording_state() {
        let mut core = make_core();
        let mut tm = TimeMachine::new(TimeMachineConfig::default());

        for _ in 0..10 {
            core.execute(Command::StepFrame).unwrap();
            tm.record_frame(&core);
        }
        assert!(wait_for_sync(&mut tm), "Worker thread failed to sync");

        tm.rewind_step(&mut core);
        tm.resume();

        assert_eq!(tm.state(), TimeMachineState::Recording);
    }

    #[test]
    fn record_frame_does_nothing_when_not_recording() {
        let mut core = make_core();
        let mut tm = TimeMachine::new(TimeMachineConfig::default());

        core.execute(Command::StepFrame).unwrap();
        tm.record_frame(&core);
        assert!(wait_for_sync(&mut tm), "Worker thread failed to sync");

        let last_frame = tm.last_recorded_frame;
        assert!(last_frame > 0);

        // Enter rewinding state
        tm.rewind_step(&mut core);
        assert!(matches!(
            tm.state(),
            TimeMachineState::Rewinding { .. } | TimeMachineState::Exhausted
        ));

        // Advance core and try to record
        core.execute(Command::StepFrame).unwrap();
        tm.record_frame(&core);

        // Frame should not be recorded
        assert_eq!(tm.last_recorded_frame, last_frame);
    }

    #[test]
    fn rewind_faster_accelerates_speed_and_clamps() {
        let mut core = make_core();
        let mut tm = TimeMachine::new(TimeMachineConfig::default());

        // Need to record at least one frame to enter Rewinding properly (otherwise it hits an early None return without setting up the cursor)
        core.execute(Command::StepFrame).unwrap();
        tm.record_frame(&core);
        core.execute(Command::StepFrame).unwrap();
        tm.record_frame(&core);
        assert!(wait_for_sync(&mut tm), "Worker thread failed to sync");

        tm.rewind_step(&mut core);
        assert!(matches!(tm.state(), TimeMachineState::Rewinding { .. }));

        // Should start at Normal
        assert_eq!(tm.cursor.as_ref().unwrap().speed, RewindSpeed::Normal);

        tm.rewind_faster();
        assert_eq!(tm.cursor.as_ref().unwrap().speed, RewindSpeed::Fast);

        tm.rewind_faster();
        assert_eq!(tm.cursor.as_ref().unwrap().speed, RewindSpeed::Faster);

        // Clamps at Faster
        tm.rewind_faster();
        assert_eq!(tm.cursor.as_ref().unwrap().speed, RewindSpeed::Faster);
    }

    #[test]
    fn history_seconds_returns_expected_value() {
        let mut core = make_core();
        let mut tm = TimeMachine::new(TimeMachineConfig::default());

        for _ in 0..60 {
            core.execute(Command::StepFrame).unwrap();
            tm.record_frame(&core);
        }

        let expected = tm.last_recorded_frame as f32 / 60.0;
        assert_eq!(tm.history_seconds(), expected);
    }

    #[test]
    fn rewind_exhausted_state_on_timeout() {
        let mut core = make_core();
        let mut tm = TimeMachine::new(TimeMachineConfig::default());

        // Record a few frames so that we have a target frame > 0
        for _ in 0..5 {
            core.execute(Command::StepFrame).unwrap();
            tm.record_frame(&core);
        }
        assert!(wait_for_sync(&mut tm), "Worker thread failed to sync");

        // Let's actually give the worker a moment to process the queue
        // to avoid race conditions.
        std::thread::sleep(std::time::Duration::from_millis(50));
        // We skip waiting for sync here to avoid race conditions with tx channel replacement

        // Artificially replace the receiver with a black hole to force a timeout
        let (_dummy_reply_tx, dummy_rx) = std::sync::mpsc::channel();
        let (dummy_work_tx, dummy_work_rx) = std::sync::mpsc::sync_channel(1);

        // Replace BOTH rx and tx to avoid panicking the worker thread when the dummy sender goes out of scope or the actual channel closes.
        tm.rx = dummy_rx;
        let old_tx = std::mem::replace(&mut tm.tx, dummy_work_tx);

        // This call will time out waiting for the dummy_rx
        let result = tm.rewind_step(&mut core);

        assert_eq!(result, None);
        assert_eq!(tm.state(), TimeMachineState::Exhausted);

        // Prevent dummy_work_rx from being dropped before we are ready
        // otherwise the worker thread might panic if it tries to reply
        // while we are swapping channels back.
        // Also wait a tiny bit to make sure it times out properly
        std::thread::sleep(std::time::Duration::from_millis(50));

        // Restore tx to allow graceful shutdown
        tm.tx = old_tx;

        // Ensure dummy_work_rx stays alive until old_tx is restored to avoid
        // "Worker thread failed to sync" panic on tx drop in some environments
        drop(dummy_work_rx);
    }
}
