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
                // No deltas anchored to oldest keyframe — drop it and any
                // orphaned deltas that preceded the next keyframe.
                self.keyframes.pop_front();
                while self.deltas.front().is_some_and(|d| d.frame_id < next_kf_id) {
                    self.deltas.pop_front();
                }
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
    fn test_reconstruct_target_less_than_oldest_keyframe_returns_none() {
        let core = make_core();
        let mut tl = CompressedTimeline::new(3, KeyframePolicy::new(1, 0)); // Promote all to KF

        tl.push(1, core.save_state());
        tl.push(2, core.save_state());
        tl.push(3, core.save_state());

        // Pushing frame 4 will pop frame 1
        tl.push(4, core.save_state());

        // tl.last_frame_id is 4. Reconstructing 1 should return None because the oldest KF is now 2.
        assert!(tl.reconstruct(1).is_none());
        assert!(tl.reconstruct(2).is_some());
    }

    #[test]
    fn test_reconstruct_ignores_delta_with_same_frame_id_as_keyframe() {
        let core = make_core();
        let mut tl = CompressedTimeline::new(3, policy());

        let mut snap = core.save_state();
        tl.keyframes.push_back(Keyframe {
            frame_id: 10,
            snapshot: snap.clone(),
        });

        // Mutate snap so the delta has a measurable effect if applied
        snap.ppu.frame_counter = 999;
        let mut delta = FrameDelta::compute(&core.save_state(), &snap);
        delta.frame_id = 10;
        tl.deltas.push_back(delta);

        tl.last_frame_id = Some(10);

        // When reconstructing frame 10, the delta for frame 10 should be IGNORED
        // because its frame_id (10) is not > kf.frame_id (10).
        let restored = tl.reconstruct(10).unwrap();
        assert_eq!(
            restored.ppu.frame_counter,
            core.save_state().ppu.frame_counter
        ); // NOT 999

        // However, if we reconstruct frame 11, we should also test that the delta for frame 10 is applied
        // Wait, if target is 11, kf is 10, delta.frame_id is 10. `delta.frame_id > kf.frame_id` (10 > 10) is false!
        // So the mutant `replace > with >=` would apply the delta here too and incorrectly apply it.
        tl.last_frame_id = Some(11);
        let restored11 = tl.reconstruct(11).unwrap();
        // Since delta.frame_id (10) is NOT > kf.frame_id (10), it should be skipped.
        // If the mutant changes > to >=, it WOULD apply the delta, turning frame_counter to 999!
        assert_eq!(
            restored11.ppu.frame_counter,
            core.save_state().ppu.frame_counter
        );
    }

    #[test]
    fn test_reconstruct_rejects_greater_than_last_frame() {
        let core = make_core();
        let mut tl = CompressedTimeline::new(3, policy());
        tl.push(5, core.save_state());

        // Reconstruct exact latest frame (5 <= 5)
        assert!(tl.reconstruct(5).is_some());

        // Reconstruct > latest frame
        assert!(tl.reconstruct(6).is_none());
    }

    #[test]
    fn empty_timeline_returns_none() {
        let tl = CompressedTimeline::new(300, policy());
        assert!(tl.is_empty());
        assert!(tl.reconstruct(0).is_none());
    }

    #[test]
    fn timeline_is_not_empty_after_push() {
        let core = make_core();
        let mut tl = CompressedTimeline::new(300, policy());
        tl.push(0, core.save_state());
        assert!(!tl.is_empty());
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

    #[test]
    fn test_reconstruct_target_is_last_keyframe() {
        let mut core = make_core();
        let mut tl = CompressedTimeline::new(300, policy());

        let snap0 = core.save_state();
        tl.push(0, snap0);

        // Push another frame to make sure it exists, but then we will reconstruct frame 0
        core.execute(Command::StepFrame).unwrap();
        let snap1 = core.save_state();
        tl.push(1, snap1);

        // Reconstruct exact keyframe (frame 0 is a keyframe)
        let restored = tl.reconstruct(0).unwrap();
        assert_eq!(restored.ppu.frame_counter, 0);
    }

    #[test]
    fn test_reconstruct_target_needs_delta_replay() {
        let mut core = make_core();
        let mut tl = CompressedTimeline::new(300, policy());

        let snap0 = core.save_state();
        tl.push(0, snap0); // kf

        core.execute(Command::StepFrame).unwrap();
        let snap1 = core.save_state();
        tl.push(1, snap1); // delta

        core.execute(Command::StepFrame).unwrap();
        let snap2 = core.save_state();
        tl.push(2, snap2); // delta

        // Target frame 1: needs to apply first delta but not second
        let restored = tl.reconstruct(1).unwrap();
        assert_eq!(restored.ppu.frame_counter, 1);

        // Target frame 2: needs to apply both deltas
        let restored2 = tl.reconstruct(2).unwrap();
        assert_eq!(restored2.ppu.frame_counter, 2);
    }

    #[test]
    fn test_prune_logic_pop_orphaned_deltas() {
        let mut core = make_core();
        // Policy: Every 1 frame is a keyframe for easy testing of pruning logic
        let mut tl = CompressedTimeline::new(3, KeyframePolicy::new(1, 0));

        // Push 4 frames into a capacity 3 timeline
        tl.push(0, core.save_state()); // KF

        core.execute(Command::StepFrame).unwrap();
        tl.push(1, core.save_state()); // KF

        core.execute(Command::StepFrame).unwrap();
        tl.push(2, core.save_state()); // KF

        core.execute(Command::StepFrame).unwrap();
        tl.push(3, core.save_state()); // KF (Trigger prune, length becomes 4, max is 3)

        // The first keyframe (frame 0) should be dropped because max_frames=3 and we pushed 4
        // Length should be exactly 3
        assert_eq!(tl.len(), 3);

        // Frame 0 should no longer be reconstructible
        assert!(tl.reconstruct(0).is_none());
        // Frame 1 should be the oldest valid frame
        assert!(tl.reconstruct(1).is_some());
    }

    #[test]
    fn test_prune_drops_oldest_keyframe_retains_newer_deltas() {
        let core = make_core();
        // Policy: Every 1 frame is a keyframe? No, we want a keyframe, then another keyframe, then a delta.
        // We'll use a custom sequence, but `push` uses `policy`.
        // We can just construct a sequence where size > policy triggers a keyframe, or we use a tiny max_frames.
        let mut tl = CompressedTimeline::new(3, KeyframePolicy::new(10, 0)); // Promote everything to keyframe if size > 0.
        // Wait, if size is 0, it doesn't promote unless interval is reached.
        // Let's manually manipulate the policy to force KF, KF, Delta, Delta...

        // Actually, we can use a trick:
        // push 0: KF0
        // push 1: KF1 (if we force it, but how?)
        // Let's just create a Timeline and manually insert items to test `prune` exactly.
        tl.keyframes.push_back(Keyframe {
            frame_id: 0,
            snapshot: core.save_state(),
        });
        tl.keyframes.push_back(Keyframe {
            frame_id: 2,
            snapshot: core.save_state(),
        });

        let snap_delta = core.save_state();
        let mut delta = FrameDelta::compute(&snap_delta, &snap_delta);
        delta.frame_id = 3;
        tl.deltas.push_back(delta);

        assert_eq!(tl.len(), 3);

        // Push one more frame to trigger prune.
        // We want to trigger the `else` branch of prune: `self.deltas.front()` is either None or its frame_id >= next_kf_id.
        // deltas.front is 3. next_kf_id is 2. So 3 >= 2. Thus it falls to `else`.
        // It should pop KF0, and keep Delta3.
        let snap4 = core.save_state();
        tl.push(4, snap4); // This will add another entry, len becomes 4, max is 3, so prune runs.

        assert_eq!(tl.len(), 3);

        // KF0 should be gone
        assert_eq!(tl.keyframes.front().unwrap().frame_id, 2);

        // Delta3 should still be there
        assert_eq!(tl.deltas.front().unwrap().frame_id, 3);
    }

    #[test]
    fn test_prune_delta_boundary_conditions() {
        let core = make_core();
        let snap = core.save_state();
        let mut tl = CompressedTimeline::new(3, policy());

        tl.keyframes.push_back(Keyframe {
            frame_id: 0,
            snapshot: snap.clone(),
        });
        tl.keyframes.push_back(Keyframe {
            frame_id: 2,
            snapshot: snap.clone(),
        });

        let mut delta = FrameDelta::compute(&snap, &snap);
        delta.frame_id = 2; // Matches next_kf_id exactly.
        tl.deltas.push_back(delta);

        let snap3 = core.save_state();
        tl.push(3, snap3); // Trigger prune. Length goes from 3 to 4, max is 3.

        // delta.frame_id (2) is NOT < next_kf_id (2). So it goes to the else branch
        // and drops KF 0. Then the inner loop while delta.frame_id < next_kf_id checks 2 < 2 (false).
        // So Delta 2 is KEPT.
        assert_eq!(tl.deltas.front().unwrap().frame_id, 2);
    }
}
