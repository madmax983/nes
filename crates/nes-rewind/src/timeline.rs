//! Anchor+delta ring buffer for the Time Machine rewind system.

use std::collections::VecDeque;

use nes_core::CoreSnapshot;

use crate::delta::FrameDelta;
use crate::policy::KeyframePolicy;

struct Keyframe {
    frame_id: u64,
    snapshot: CoreSnapshot,
}

/// Rolling history of emulator states as anchor keyframes + compressed deltas.
///
/// Stores full [`CoreSnapshot`] keyframes at adaptive intervals and
/// [`FrameDelta`]s between them. Old history is pruned automatically when
/// `max_frames` is exceeded.
pub struct CompressedTimeline {
    keyframes: VecDeque<Keyframe>,
    deltas: VecDeque<FrameDelta>,
    max_frames: u64,
    policy: KeyframePolicy,
    last_snapshot: Option<CoreSnapshot>,
    last_frame_id: Option<u64>,
}

impl CompressedTimeline {
    /// Create a new timeline with the given capacity and keyframe policy.
    pub fn new(max_frames: u64, policy: KeyframePolicy) -> Self {
        Self {
            keyframes: VecDeque::new(),
            deltas: VecDeque::new(),
            max_frames,
            policy,
            last_snapshot: None,
            last_frame_id: None,
        }
    }

    /// Push a new frame into the timeline.
    ///
    /// The first frame is always stored as a keyframe. Subsequent frames are
    /// stored as deltas unless [`KeyframePolicy`] promotes them to keyframes.
    pub fn push(&mut self, frame_id: u64, snapshot: CoreSnapshot) {
        if let Some(prev) = self.last_snapshot.take() {
            let mut delta = FrameDelta::compute(&prev, &snapshot);
            // Override with the caller-supplied frame_id — FrameDelta::compute
            // uses ppu.frame_counter which may lag behind the logical frame id.
            delta.frame_id = frame_id;
            let size = delta.compressed_size();

            if self.policy.should_promote(size) {
                self.keyframes.push_back(Keyframe {
                    frame_id,
                    snapshot: snapshot.clone(),
                });
            } else {
                self.deltas.push_back(delta);
            }
        } else {
            // First frame always anchors.
            self.keyframes.push_back(Keyframe {
                frame_id,
                snapshot: snapshot.clone(),
            });
        }

        self.last_snapshot = Some(snapshot);
        self.last_frame_id = Some(frame_id);
        self.prune();
    }

    /// Reconstruct the snapshot at `target_frame_id`.
    ///
    /// Finds the nearest keyframe at or before `target_frame_id`, then
    /// replays deltas forward. Returns `None` if the frame has been pruned,
    /// was never recorded, or is beyond the last recorded frame.
    #[must_use]
    pub fn reconstruct(&self, target_frame_id: u64) -> Option<CoreSnapshot> {
        // Reject frames we've never seen.
        if self.last_frame_id.is_none_or(|last| target_frame_id > last) {
            return None;
        }

        // Latest keyframe at or before target.
        let kf = self
            .keyframes
            .iter()
            .rev()
            .find(|kf| kf.frame_id <= target_frame_id)?;
        let mut snap = kf.snapshot.clone();

        if kf.frame_id == target_frame_id {
            return Some(snap);
        }

        // Replay deltas between keyframe and target.
        for delta in &self.deltas {
            if delta.frame_id > kf.frame_id && delta.frame_id <= target_frame_id {
                delta.apply(&mut snap);
            }
        }
        Some(snap)
    }

    /// Total number of recorded entries (keyframes + deltas).
    #[must_use]
    pub fn len(&self) -> usize {
        self.keyframes.len() + self.deltas.len()
    }

    /// Returns `true` if no frames have been recorded yet.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.keyframes.is_empty() && self.deltas.is_empty()
    }

    fn prune(&mut self) {
        while self.len() as u64 > self.max_frames {
            // Where does the second-oldest keyframe begin?
            let next_kf_id = self
                .keyframes
                .get(1)
                .map(|kf| kf.frame_id)
                .unwrap_or(u64::MAX);

            if self.deltas.front().is_some_and(|d| d.frame_id < next_kf_id) {
                // Advance the oldest keyframe by absorbing the first delta into
                // it in-place. This removes one delta (net -1) without breaking
                // the anchor chain.
                let delta = self.deltas.pop_front().unwrap();
                if let Some(oldest) = self.keyframes.front_mut() {
                    delta.apply(&mut oldest.snapshot);
                    oldest.frame_id = delta.frame_id;
                }
            } else {
                // No deltas anchored to oldest keyframe — drop it.
                // (Any deltas would have frame_id >= next_kf_id, so they are kept).
                self.keyframes.pop_front();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nes_core::{Command, NesCore};

    fn make_core() -> NesCore {
        let mut core = NesCore::new();
        core.load_cpu_bytes(0xC000, &[0xEA, 0x4C, 0x00, 0xC0]);
        core
    }

    fn policy() -> KeyframePolicy {
        KeyframePolicy::new(10, 2048)
    }

    #[test]
    fn empty_timeline_returns_none() {
        let tl = CompressedTimeline::new(300, policy());
        assert!(tl.is_empty());
        assert!(tl.reconstruct(0).is_none());
    }

    #[test]
    fn first_push_is_keyframe_and_reconstructs() {
        let core = make_core();
        let snap = core.save_state();
        let mut tl = CompressedTimeline::new(300, policy());
        tl.push(0, snap.clone());

        assert_eq!(tl.len(), 1);
        let got = tl.reconstruct(0).unwrap();
        assert_eq!(got.ppu.frame_counter, snap.ppu.frame_counter);
    }

    #[test]
    fn reconstruct_unknown_frame_returns_none() {
        let core = make_core();
        let mut tl = CompressedTimeline::new(300, policy());
        tl.push(0, core.save_state());
        assert!(tl.reconstruct(999).is_none());
    }

    #[test]
    fn push_prunes_when_over_capacity() {
        let core = make_core();
        let snap = core.save_state();
        let mut tl = CompressedTimeline::new(5, policy());
        for i in 0..10u64 {
            tl.push(i, snap.clone());
        }
        assert!(tl.len() <= 5);
        // Oldest frames should be gone.
        assert!(tl.reconstruct(0).is_none());
        // Most recent should still be available.
        assert!(tl.reconstruct(9).is_some());
    }

    #[test]
    fn reconstruct_after_step_frame_restores_earlier_state() {
        let mut core = make_core();
        let mut tl = CompressedTimeline::new(300, policy());

        let snap0 = core.save_state();
        tl.push(0, snap0);

        core.execute(Command::StepFrame).unwrap();
        let snap1 = core.save_state();
        let frame1 = snap1.ppu.frame_counter;
        tl.push(frame1, snap1);

        core.execute(Command::StepFrame).unwrap();
        let snap2 = core.save_state();
        let frame2 = snap2.ppu.frame_counter;
        tl.push(frame2, snap2);

        // Reconstruct frame 0 — should have frame_counter 0.
        let restored = tl.reconstruct(0).unwrap();
        assert_eq!(restored.ppu.frame_counter, 0);
    }
}

#[cfg(test)]
mod tests_sentry_timeline {
    use super::*;
    use crate::policy::KeyframePolicy;
    use nes_core::NesCore;

    fn make_core() -> NesCore {
        let mut core = NesCore::new();
        let _ = core.load_ines_rom(&[0; 16]);
        core
    }

    #[test]
    fn test_is_empty_false_when_only_keyframes_present() {
        let mut timeline = CompressedTimeline::new(10, KeyframePolicy::new(60, 1024));
        let core = make_core();
        timeline.keyframes.push_back(Keyframe {
            frame_id: 0,
            snapshot: core.save_state(),
        });
        assert!(
            !timeline.is_empty(),
            "Timeline should not be empty if it has a keyframe"
        );
    }

    #[test]
    fn test_is_empty_false_when_only_deltas_present() {
        let mut timeline = CompressedTimeline::new(10, KeyframePolicy::new(60, 1024));
        let old = make_core().save_state();
        let new = make_core().save_state();
        timeline
            .deltas
            .push_back(crate::delta::FrameDelta::compute(&old, &new));
        assert!(
            !timeline.is_empty(),
            "Timeline should not be empty if it has a delta"
        );
    }

    #[test]
    fn test_prune_boundary_condition() {
        let mut timeline = CompressedTimeline::new(2, KeyframePolicy::new(60, 1024)); // Max frames = 2
        let core = make_core();
        timeline.push(0, core.save_state()); // len = 1
        timeline.push(1, core.save_state()); // len = 2

        assert_eq!(timeline.len(), 2);
        timeline.prune(); // Should not prune
        assert_eq!(timeline.len(), 2);

        timeline.push(2, core.save_state()); // len = 3 -> should auto prune down to 2
        assert_eq!(timeline.len(), 2, "Push should auto-prune to max length 2");
    }

    #[test]
    fn test_prune_orphan_deltas_exclusion() {
        let mut timeline = CompressedTimeline::new(3, KeyframePolicy::new(60, 1024));
        let core = make_core();
        let snap = core.save_state();

        timeline.keyframes.push_back(Keyframe {
            frame_id: 0,
            snapshot: snap.clone(),
        });
        timeline.keyframes.push_back(Keyframe {
            frame_id: 3,
            snapshot: snap.clone(),
        });

        // Delta 0, 1, 2
        let mut delta1 = crate::delta::FrameDelta::compute(&snap, &snap);
        delta1.frame_id = 1;
        let mut delta2 = crate::delta::FrameDelta::compute(&snap, &snap);
        delta2.frame_id = 2;
        timeline.deltas.push_back(delta1);
        timeline.deltas.push_back(delta2);

        // This makes len() = 4 > max_frames(3)
        // prune() should be called. It sees deltas before next_kf_id (3), so it absorbs delta 1
        timeline.prune();

        assert_eq!(timeline.keyframes.len(), 2);
        assert_eq!(
            timeline.deltas.len(),
            1,
            "Should have absorbed exactly 1 delta"
        );
        assert_eq!(
            timeline.keyframes.front().unwrap().frame_id,
            1,
            "Oldest keyframe should advance to frame 1"
        );
    }

    #[test]
    fn test_reconstruct_exact_match() {
        let mut timeline = CompressedTimeline::new(10, KeyframePolicy::new(60, 1024));
        let core = make_core();
        timeline.push(0, core.save_state());
        timeline.push(1, core.save_state());

        let reconstructed = timeline.reconstruct(0);
        assert!(reconstructed.is_some());

        // Also check missing boundary
        let reconstructed2 = timeline.reconstruct(2);
        assert!(reconstructed2.is_none());
    }

    #[test]
    fn test_reconstruct_greater_than_or_equal_mutant_real() {
        let mut timeline = CompressedTimeline::new(10, KeyframePolicy::new(60, 1024));
        let core = make_core();

        let snap0 = core.save_state();
        let mut snap1 = snap0.clone();
        snap1.ppu.frame_counter = 1;

        let mut snap_wrong = snap0.clone();
        snap_wrong.ppu.frame_counter = 999;

        timeline.keyframes.push_back(Keyframe {
            frame_id: 1,
            snapshot: snap1.clone(),
        });
        timeline.last_frame_id = Some(2);

        // A delta for frame 1 that would set frame_counter to 999.
        let mut delta1 = crate::delta::FrameDelta::compute(&snap0, &snap_wrong);
        delta1.frame_id = 1;
        timeline.deltas.push_back(delta1);

        let mut snap2 = snap1.clone();
        snap2.ppu.frame_counter = 2;
        let mut delta2 = crate::delta::FrameDelta::compute(&snap1, &snap2);
        delta2.frame_id = 2;
        timeline.deltas.push_back(delta2);

        // Reconstruct frame 2. Keyframe is 1.
        // It should apply delta 2. It should NOT apply delta 1 (because delta1.frame_id = 1, kf.frame_id = 1, 1 > 1 is false).
        // If mutated to >=, 1 >= 1 is true, so delta1 is applied, setting frame_counter to 999.
        let restored2 = timeline.reconstruct(2).unwrap();
        assert_eq!(
            restored2.ppu.frame_counter, 2,
            "If > was mutated to >=, this would be 999"
        );
    }

    #[test]
    fn test_is_empty_mutants() {
        let mut timeline = CompressedTimeline::new(10, KeyframePolicy::new(60, 1024));
        let core = make_core();

        assert!(timeline.is_empty());

        timeline.keyframes.push_back(Keyframe {
            frame_id: 0,
            snapshot: core.save_state(),
        });
        assert!(
            !timeline.is_empty(),
            "Timeline should not be empty with just a keyframe"
        );

        timeline.keyframes.clear();
        let old = core.save_state();
        let new = core.save_state();
        timeline
            .deltas
            .push_back(crate::delta::FrameDelta::compute(&old, &new));
        assert!(
            !timeline.is_empty(),
            "Timeline should not be empty with just a delta"
        );
    }

    #[test]
    fn test_prune_less_than_mutants_real() {
        // Line 129: `if self.deltas.front().is_some_and(|d| d.frame_id < next_kf_id)`
        let mut timeline = CompressedTimeline::new(3, KeyframePolicy::new(60, 1024));
        let core = make_core();
        let snap = core.save_state();

        timeline.keyframes.push_back(Keyframe {
            frame_id: 0,
            snapshot: snap.clone(),
        });
        timeline.keyframes.push_back(Keyframe {
            frame_id: 3,
            snapshot: snap.clone(),
        });

        let mut delta = crate::delta::FrameDelta::compute(&snap, &snap);
        delta.frame_id = 3;
        timeline.deltas.push_back(delta);

        let mut delta2 = crate::delta::FrameDelta::compute(&snap, &snap);
        delta2.frame_id = 4;
        timeline.deltas.push_back(delta2);

        timeline.prune();

        assert_eq!(
            timeline.deltas.len(),
            2,
            "Delta 3 should not be popped if < next_kf_id is strictly checked"
        );
        assert_eq!(timeline.deltas.front().unwrap().frame_id, 3);
    }

    #[test]
    fn test_prune_less_than_mutants_real_gt_eq() {
        let mut timeline = CompressedTimeline::new(3, KeyframePolicy::new(60, 1024));
        let core = make_core();
        let snap = core.save_state();

        timeline.keyframes.push_back(Keyframe {
            frame_id: 0,
            snapshot: snap.clone(),
        });
        timeline.keyframes.push_back(Keyframe {
            frame_id: 3,
            snapshot: snap.clone(),
        });

        let mut delta = crate::delta::FrameDelta::compute(&snap, &snap);
        delta.frame_id = 4; // strictly >
        timeline.deltas.push_back(delta);

        let mut delta2 = crate::delta::FrameDelta::compute(&snap, &snap);
        delta2.frame_id = 5;
        timeline.deltas.push_back(delta2);

        timeline.prune();

        assert_eq!(
            timeline.deltas.len(),
            2,
            "Delta 4 should not be popped if < next_kf_id is strictly checked"
        );
        assert_eq!(timeline.deltas.front().unwrap().frame_id, 4);
    }
}
